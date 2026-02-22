use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::Context;
use async_stream::stream;
use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum_server::tls_rustls::RustlsConfig;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use datastar::prelude::PatchSignals;
use rcgen::generate_simple_self_signed;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::compression::CompressionLayer;
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};

use crate::ControlMessage;
use crate::SharedStreamState;
use crate::config::WebTransport;
use crate::frame::{FrameEvent, FrameKind};
use crate::telemetry::Telemetry;

const INDEX_HTML: &str = r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>datastar-matrix</title>
  <script type="module" src="https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.0-RC.7/bundles/datastar.js"></script>
  <style>
    :root {
      --bg: #000;
      --g0: rgb(160, 255, 160);
      --g1: rgb(0, 195, 0);
      --g2: rgb(0, 170, 0);
      --g3: rgb(0, 145, 0);
      --g4: rgb(0, 95, 0);
      --g5: rgb(0, 20, 0);
    }
    body { margin: 0; background: var(--bg); color: var(--g1); font-family: monospace; }
    .wrap { position: relative; width: 100vw; height: 100svh; overflow: hidden; }
    #matrix {
      margin: 0;
      display: block;
      width: 100%;
      height: 100%;
    }
  </style>
</head>
<body>
  <div class="wrap">
    <canvas
      id="matrix"
      data-on:mousedown="$x = window.__matrixDatastar.glitchCoord(evt, 'x'); $y = window.__matrixDatastar.glitchCoord(evt, 'y'); @post('/cmd/glitch')"
    ></canvas>
    <div
      id="ds"
      data-signals="{frameId: 0, speed: 16, sentMs: 0, connected: false, packedB64: '', clientId: (window.crypto && window.crypto.randomUUID ? window.crypto.randomUUID() : ('c-' + Math.random().toString(36).slice(2))), cols: Math.max(1, Math.ceil(window.innerWidth / 10)), rows: Math.max(1, Math.floor((window.visualViewport ? window.visualViewport.height : window.innerHeight) / 20)), width: Math.max(1, Math.ceil(window.innerWidth / 10)), height: Math.max(1, Math.floor((window.visualViewport ? window.visualViewport.height : window.innerHeight) / 20)), x: 0, y: 0}"
      data-init="@patch('/events', {clientId: $clientId, cols: $cols, rows: $rows})"
      data-effect="window.__matrixDatastar.onFrame($frameId, $speed, $sentMs, $packedB64, $width, $height)"
      data-on:keydown__window="if (evt.key === '?') { window.__matrixDatastar.toggleStats(); } else if (evt.key === ' ') { evt.preventDefault(); @post('/cmd/toggle_pause', {contentType: 'form'}); } else if (evt.key === '+' || evt.key === '=') { @post('/cmd/speed_up', {contentType: 'form'}); } else if (evt.key === '-' || evt.key === '_') { @post('/cmd/speed_down', {contentType: 'form'}); } else if (evt.key === '0') { @post('/cmd/reset_speed', {contentType: 'form'}); }"
      data-on:resize__window__debounce.500ms="$cols = Math.max(1, Math.ceil(window.innerWidth / 10)); $rows = Math.max(1, Math.floor((window.visualViewport ? window.visualViewport.height : window.innerHeight) / 20)); $width = $cols; $height = $rows; window.__matrixDatastar.onResize($cols, $rows); @post('/cmd/resize')"
    ></div>
  </div>
  <script>
    const CELL_W = 10;
    const CELL_H = 20;
    const FONT = '12pt "Osaka-Mono", "MS Gothic", "Noto Sans Mono CJK JP", "Menlo", "Monaco", monospace';
    const GLYPHS = [
      ' ','0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z',
      'ｱ','ｲ','ｳ','ｴ','ｵ','ｶ','ｷ','ｸ','ｹ','ｺ','ｻ','ｼ','ｽ','ｾ','ｿ','ﾀ','ﾁ','ﾂ','ﾃ','ﾄ','ﾅ','ﾆ','ﾇ','ﾈ','ﾉ','ﾊ','ﾋ','ﾌ','ﾍ','ﾎ','ﾏ','ﾐ','ﾑ','ﾒ','ﾓ','ﾔ','ﾕ','ﾖ','ﾗ','ﾘ','ﾙ','ﾚ','ﾛ','ﾜ','ﾝ'
    ];
    const matrix = document.getElementById('matrix');
    const ctx = matrix.getContext('2d', { alpha: false, desynchronized: true });
    const SIGNAL_LOST_ART = ['<<< [ SIGNAL LOST ] >>>'];
    let timeoutHandle;
    let disconnected = false;
    let showStats = false;
    let lastFrameId = 0;
    let lastRenderedFrame = 0;
    let lastSpeed = 16;
    let lastSentMs = 0;
    let fps = 0;
    let fpsLastFrame = 0;
    let fpsLastAt = performance.now();
    let latencySamples = [];
    let canvasW = 0;
    let canvasH = 0;
    let canvasDpr = 0;
    let lastResizeCols = 0;
    let lastResizeRows = 0;
    let lastViewportW = 0;
    let lastViewportH = 0;
    const lastStatsRect = { x: 0, y: 0, w: 0 };

    function viewportCols() {
      return Math.max(1, lastViewportW || Math.ceil(window.innerWidth / CELL_W));
    }

    function viewportRows() {
      const vh = (window.visualViewport ? window.visualViewport.height : window.innerHeight);
      return Math.max(1, lastViewportH || Math.floor(vh / CELL_H));
    }

    function scheduleDisconnect() {
      clearTimeout(timeoutHandle);
      timeoutHandle = setTimeout(() => {
        disconnected = true;
        const width = viewportCols();
        const height = viewportRows();
        renderSignalLost(width, height);
      }, 3000);
    }

    function buildStatsText(frameId, speed, sentMs) {
      if (!showStats) return '';
      const now = performance.now();
      if (frameId > lastFrameId && now - fpsLastAt >= 400) {
        fps = (frameId - fpsLastFrame) / ((now - fpsLastAt) / 1000.0);
        fpsLastFrame = frameId;
        fpsLastAt = now;
      }
      const nowWall = Date.now();
      const oneWay = Math.max(0, nowWall - sentMs);
      latencySamples.push({ t: nowWall, v: oneWay });
      while (latencySamples.length && (nowWall - latencySamples[0].t) > 250) latencySamples.shift();
      let latencyMs = 0;
      if (latencySamples.length) {
        const total = latencySamples.reduce((acc, s) => acc + s.v, 0);
        latencyMs = total / latencySamples.length;
      }
      return `[ latency:${latencyMs.toFixed(1)}ms  speed:${speed}  frame:${frameId}  fps:${fps.toFixed(1)} ]`;
    }

    function drawStatsOverlay(text, width, height) {
      if (!ctx || !width || !height) return;
      if (lastStatsRect.w > 0) {
        ctx.fillStyle = '#000';
        ctx.fillRect(lastStatsRect.x * CELL_W, lastStatsRect.y * CELL_H, lastStatsRect.w * CELL_W, CELL_H);
        lastStatsRect.w = 0;
      }
      if (!text) return;
      const x0 = Math.max(0, width - text.length);
      const y = Math.max(0, height - 1);
      ctx.fillStyle = '#000';
      ctx.fillRect(x0 * CELL_W, y * CELL_H, text.length * CELL_W, CELL_H);
      ctx.fillStyle = 'rgb(160,255,160)';
      for (let i = 0; i < text.length; i++) {
        const ch = text[i];
        if (ch !== ' ') {
          ctx.fillText(ch, (x0 + i) * CELL_W, y * CELL_H);
        }
      }
      lastStatsRect.x = x0;
      lastStatsRect.y = y;
      lastStatsRect.w = text.length;
    }

    function classFromLum(lum) {
      if (lum >= 235) return 0;
      if (lum >= 180) return 1;
      if (lum >= 150) return 2;
      if (lum >= 120) return 3;
      if (lum >= 80) return 4;
      return 5;
    }

    function decodePacked(packedB64) {
      if (!packedB64) return new Uint8Array(0);
      const bin = atob(packedB64);
      const out = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
      return out;
    }

    function ensureCanvasSize(width, height) {
      const dpr = window.devicePixelRatio || 1;
      if (width !== canvasW || height !== canvasH || dpr !== canvasDpr) {
        matrix.width = Math.max(1, Math.floor(width * CELL_W * dpr));
        matrix.height = Math.max(1, Math.floor(height * CELL_H * dpr));
        matrix.style.width = `${Math.max(1, width * CELL_W)}px`;
        matrix.style.height = `${Math.max(1, height * CELL_H)}px`;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.textBaseline = 'top';
        ctx.textAlign = 'left';
        ctx.font = FONT;
        canvasW = width;
        canvasH = height;
        canvasDpr = dpr;
      }
    }

    function clearCanvas(width, height) {
      ctx.fillStyle = '#000';
      ctx.fillRect(0, 0, width * CELL_W, height * CELL_H);
    }

    function renderSignalLost(width, height) {
      if (!ctx || !width || !height) return;
      ensureCanvasSize(width, height);
      const artH = SIGNAL_LOST_ART.length;
      const artW = SIGNAL_LOST_ART.reduce((m, row) => Math.max(m, row.length), 0);
      const startX = Math.floor((width - artW) / 2);
      const startY = Math.floor((height - artH) / 2);
      const margin = 1;
      const blankX = Math.max(0, startX - margin);
      const blankY = Math.max(0, startY - margin);
      const blankW = Math.min(width - blankX, artW + margin * 2);
      const blankH = Math.min(height - blankY, artH + margin * 2);
      ctx.fillStyle = '#000';
      ctx.fillRect(blankX * CELL_W, blankY * CELL_H, blankW * CELL_W, blankH * CELL_H);

      for (let y = 0; y < artH; y++) {
        const row = SIGNAL_LOST_ART[y];
        let leftAngleSeen = 0;
        let rightAngleSeen = 0;
        for (let x = 0; x < row.length; x++) {
          const ch = row[x];
          if (ch !== ' ') {
            if (ch === '[' || ch === ']') {
              ctx.fillStyle = 'rgb(160,255,160)';
            } else if (ch === '<') {
              const shades = ['rgb(0,95,0)', 'rgb(0,145,0)', 'rgb(0,195,0)'];
              ctx.fillStyle = shades[Math.min(leftAngleSeen, shades.length - 1)];
              leftAngleSeen += 1;
            } else if (ch === '>') {
              const shades = ['rgb(0,195,0)', 'rgb(0,145,0)', 'rgb(0,95,0)'];
              ctx.fillStyle = shades[Math.min(rightAngleSeen, shades.length - 1)];
              rightAngleSeen += 1;
            } else {
              ctx.fillStyle = 'rgb(0,195,0)';
            }
            ctx.fillText(ch, (startX + x) * CELL_W, (startY + y) * CELL_H);
          }
        }
      }
    }

    function renderCanvas(packedB64, width, height, frameId) {
      if (!ctx || !packedB64 || !width || !height) {
        return;
      }
      if (frameId <= lastRenderedFrame) return;
      ensureCanvasSize(width, height);
      clearCanvas(width, height);
      const packed = decodePacked(packedB64);
      const needed = width * height * 2;
      if (packed.length < needed) return;
      const colors = ['rgb(160,255,160)', 'rgb(0,195,0)', 'rgb(0,170,0)', 'rgb(0,145,0)', 'rgb(0,95,0)', 'rgb(0,20,0)'];
      let lastClass = -1;
      let p = 0;
      for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
          const glyph = packed[p];
          const lum = packed[p + 1];
          p += 2;
          const ch = GLYPHS[glyph] || ' ';
          if (ch !== ' ') {
            const cls = classFromLum(lum);
            if (cls !== lastClass) {
              ctx.fillStyle = colors[cls];
              lastClass = cls;
            }
            ctx.fillText(ch, x * CELL_W, y * CELL_H);
          }
        }
      }
      lastRenderedFrame = frameId;
    }

    window.__matrixDatastar = {
      onFrame(frameId, speed, sentMs, packedB64, width, height) {
        frameId = Number(frameId || 0);
        speed = Number(speed || 16);
        sentMs = Number(sentMs || 0);
        width = Number(width || 0);
        height = Number(height || 0);
        disconnected = false;
        renderCanvas(packedB64 || '', width, height, frameId);
        drawStatsOverlay(buildStatsText(frameId, speed, sentMs), viewportCols(), viewportRows());
        scheduleDisconnect();
        lastFrameId = frameId;
        lastSpeed = speed;
        lastSentMs = sentMs;
      },
      toggleStats() {
        showStats = !showStats;
        const width = viewportCols();
        const height = viewportRows();
        drawStatsOverlay(buildStatsText(lastFrameId, lastSpeed, lastSentMs), width, height);
      },
      onResize(cols, rows) {
        cols = Number(cols || 1);
        rows = Number(rows || 1);
        if (cols === lastResizeCols && rows === lastResizeRows) return;
        lastResizeCols = cols;
        lastResizeRows = rows;
        lastViewportW = cols;
        lastViewportH = rows;
        ensureCanvasSize(cols, rows);
        if (disconnected) {
          renderSignalLost(cols, rows);
        } else {
          drawStatsOverlay(buildStatsText(lastFrameId, lastSpeed, lastSentMs), cols, rows);
        }
      },
      glitchCoord(evt, axis) {
        const rect = matrix.getBoundingClientRect();
        const px = evt.clientX - rect.left;
        const py = evt.clientY - rect.top;
        let x = Math.floor(px / CELL_W);
        let y = Math.floor(py / CELL_H);
        const maxX = Math.max(0, canvasW - 1);
        const maxY = Math.max(0, canvasH - 1);
        x = Math.max(0, Math.min(maxX, x));
        y = Math.max(0, Math.min(maxY, y));
        return axis === 'y' ? y : x;
      },
    };
    scheduleDisconnect();
  </script>
</body>
</html>"#;

#[derive(Clone)]
struct AppState {
    shared: Arc<SharedStreamState>,
    telemetry: Arc<Telemetry>,
    shutdown: CancellationToken,
    resize_tracker: Arc<ResizeTracker>,
}

#[derive(Default)]
struct ResizeTracker {
    clients: Mutex<HashMap<String, (u16, u16)>>,
}

impl ResizeTracker {
    fn set_client_viewport(&self, client_id: &str, cols: u16, rows: u16) -> (u16, u16) {
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        clients.insert(client_id.to_string(), (cols.max(1), rows.max(1)));
        max_dimensions(&clients)
    }

    fn remove_client(&self, client_id: &str) -> (u16, u16) {
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        clients.remove(client_id);
        max_dimensions(&clients)
    }
}

fn max_dimensions(clients: &HashMap<String, (u16, u16)>) -> (u16, u16) {
    clients
        .values()
        .fold((0u16, 0u16), |(max_w, max_h), (w, h)| {
            (max_w.max(*w), max_h.max(*h))
        })
}

fn normalize_client_id(client_id: Option<String>) -> Option<String> {
    client_id.and_then(|id| {
        let id = id.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_string())
        }
    })
}

pub struct WebTask {
    pub handle: JoinHandle<anyhow::Result<()>>,
    pub shutdown: CancellationToken,
    pub port: u16,
    pub scheme: &'static str,
}

fn bind_port(requested_port: Option<u16>) -> u16 {
    requested_port.unwrap_or(0)
}

fn bind_host(public_server: bool) -> Ipv4Addr {
    if public_server {
        Ipv4Addr::UNSPECIFIED
    } else {
        Ipv4Addr::LOCALHOST
    }
}

fn first_frame_for_client(snapshot: &FrameEvent, trigger: &FrameEvent) -> FrameEvent {
    FrameEvent {
        frame_id: trigger.frame_id,
        speed_step: trigger.speed_step,
        width: snapshot.width,
        height: snapshot.height,
        kind: FrameKind::Keyframe,
        cells: snapshot.cells.clone(),
    }
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn apply_cell_buffers(frame: &FrameEvent, glyph_buffer: &mut Vec<u8>, lum_buffer: &mut Vec<u8>) {
    let needed = frame.width as usize * frame.height as usize;
    if glyph_buffer.len() != needed {
        *glyph_buffer = vec![0; needed];
    }
    if lum_buffer.len() != needed {
        *lum_buffer = vec![0; needed];
    }
    if frame.kind == FrameKind::Keyframe {
        glyph_buffer.fill(0);
        lum_buffer.fill(0);
    }
    for cell in &frame.cells {
        if cell.x < frame.width && cell.y < frame.height {
            let idx = cell.y as usize * frame.width as usize + cell.x as usize;
            if idx < glyph_buffer.len() {
                glyph_buffer[idx] = cell.glyph;
                lum_buffer[idx] = cell.lum;
            }
        }
    }
}

fn datastar_signal_event(
    frame: &FrameEvent,
    glyph_buffer: &mut Vec<u8>,
    lum_buffer: &mut Vec<u8>,
    packed_buffer: &mut Vec<u8>,
) -> Event {
    apply_cell_buffers(frame, glyph_buffer, lum_buffer);
    let cells = glyph_buffer.len();
    let needed = cells * 2;
    if packed_buffer.len() != needed {
        *packed_buffer = vec![0; needed];
    }
    for i in 0..cells {
        let p = i * 2;
        packed_buffer[p] = glyph_buffer[i];
        packed_buffer[p + 1] = lum_buffer[i];
    }
    let packed_b64 = STANDARD.encode(&*packed_buffer);
    let signals = json!({
        "frameId": frame.frame_id,
        "speed": frame.speed_step,
        "sentMs": unix_ms(),
        "connected": true,
        "packedB64": packed_b64,
        "width": frame.width,
        "height": frame.height,
    })
    .to_string();
    PatchSignals::new(signals).write_as_axum_sse_event()
}

fn clamp_glitch_to_frame(glitch: GlitchBody, frame: &FrameEvent) -> (u16, u16) {
    let max_x = frame.width.saturating_sub(1) as i32;
    let max_y = frame.height.saturating_sub(1) as i32;
    (
        glitch.x.clamp(0, max_x) as u16,
        glitch.y.clamp(0, max_y) as u16,
    )
}

fn ensure_rustls_provider() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn generate_dev_tls_pem() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
        "ironhide.local".to_string(),
    ];
    let cert = generate_simple_self_signed(names).context("failed to generate dev tls cert")?;
    Ok((
        cert.cert.pem().into_bytes(),
        cert.key_pair.serialize_pem().into_bytes(),
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn spawn_server(
    token: CancellationToken,
    requested_port: Option<u16>,
    public_server: bool,
    transport: WebTransport,
    tx: broadcast::Sender<FrameEvent>,
    latest: Arc<RwLock<FrameEvent>>,
    resize_tx: mpsc::UnboundedSender<(u16, u16)>,
    control_tx: mpsc::UnboundedSender<ControlMessage>,
    telemetry: Arc<Telemetry>,
) -> anyhow::Result<WebTask> {
    let shutdown = token.child_token();

    let bind_addr = SocketAddr::from((bind_host(public_server), bind_port(requested_port)));
    let std_listener = std::net::TcpListener::bind(bind_addr).context("web bind failed")?;
    std_listener
        .set_nonblocking(true)
        .context("failed to set non-blocking listener")?;
    let port = std_listener
        .local_addr()
        .context("could not inspect local address")?
        .port();
    let app_state = AppState {
        shared: Arc::new(SharedStreamState {
            tx,
            latest,
            resize_tx,
            control_tx,
        }),
        telemetry,
        shutdown: shutdown.clone(),
        resize_tracker: Arc::new(ResizeTracker::default()),
    };
    let compression_predicate = SizeAbove::default()
        .and(NotForContentType::GRPC)
        .and(NotForContentType::IMAGES);

    // CQRS transport split: commands are accepted on /cmd/* (204 ack),
    // while observable UI/query state is streamed on /events as SSE.
    let app = Router::new()
        .route("/", get(index))
        .route("/events", get(events_datastar).patch(events_datastar_patch))
        .route("/cmd/{op}", post(cmd))
        .with_state(app_state)
        .layer(CompressionLayer::new().compress_when(compression_predicate));

    let tls_config = match &transport {
        WebTransport::Http => None,
        WebTransport::HttpsProvided {
            cert_path,
            key_path,
        } => {
            ensure_rustls_provider();
            Some(
                RustlsConfig::from_pem_file(cert_path.clone(), key_path.clone())
                    .await
                    .context("failed to load tls cert/key")?,
            )
        }
        WebTransport::HttpsAuto => {
            ensure_rustls_provider();
            let (cert_pem, key_pem) = generate_dev_tls_pem()?;
            Some(
                RustlsConfig::from_pem(cert_pem, key_pem)
                    .await
                    .context("failed to load generated dev tls cert/key")?,
            )
        }
    };

    let scheme = match &transport {
        WebTransport::Http => "http",
        WebTransport::HttpsProvided { .. } | WebTransport::HttpsAuto => "https",
    };

    let handle = tokio::spawn(async move {
        match transport {
            WebTransport::Http => {
                let listener = TcpListener::from_std(std_listener)
                    .context("failed to create tokio listener")?;
                axum::serve(listener, app)
                    .with_graceful_shutdown(shutdown.cancelled_owned())
                    .await
                    .context("web serve failed")?;
            }
            WebTransport::HttpsProvided { .. } | WebTransport::HttpsAuto => {
                let server_handle = axum_server::Handle::new();
                let shutdown_handle = server_handle.clone();
                let shutdown_task = tokio::spawn(async move {
                    shutdown.cancelled().await;
                    shutdown_handle.graceful_shutdown(Some(Duration::from_secs(2)));
                });

                axum_server::from_tcp_rustls(
                    std_listener,
                    tls_config.expect("tls config must be present in https mode"),
                )
                .handle(server_handle)
                .serve(app.into_make_service())
                .await
                .context("web tls serve failed")?;
                shutdown_task.abort();
            }
        }
        Ok(())
    });

    Ok(WebTask {
        handle,
        shutdown: token,
        port,
        scheme,
    })
}

async fn index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn events_datastar(
    State(state): State<AppState>,
    Query(viewport): Query<ViewportQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    events_datastar_inner(state, viewport.client_id, viewport.cols, viewport.rows)
}

async fn events_datastar_patch(
    State(state): State<AppState>,
    Json(viewport): Json<ViewportBody>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    events_datastar_inner(state, viewport.client_id, viewport.cols, viewport.rows)
}

fn events_datastar_inner(
    state: AppState,
    client_id: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let client_id = normalize_client_id(client_id);
    if let (Some(cols), Some(rows)) = (cols, rows) {
        if let Some(client_id) = client_id.as_deref() {
            let (max_w, max_h) =
                state
                    .resize_tracker
                    .set_client_viewport(client_id, cols.max(1), rows.max(1));
            let _ = state.shared.resize_tx.send((max_w, max_h));
        } else {
            let _ = state.shared.resize_tx.send((cols.max(1), rows.max(1)));
        }
    }
    state.telemetry.inc_clients();
    let mut rx = state.shared.tx.subscribe();
    let latest = state.shared.latest.clone();
    let telemetry = state.telemetry.clone();
    let shutdown = state.shutdown.clone();
    let resize_tracker = state.resize_tracker.clone();
    let resize_tx = state.shared.resize_tx.clone();

    let stream = stream! {
        struct ClientGuard {
            telemetry: Arc<Telemetry>,
            resize_tracker: Arc<ResizeTracker>,
            resize_tx: mpsc::UnboundedSender<(u16, u16)>,
            client_id: Option<String>,
        }
        impl Drop for ClientGuard {
            fn drop(&mut self) {
                self.telemetry.dec_clients();
                if let Some(client_id) = self.client_id.as_deref() {
                    let (max_w, max_h) = self.resize_tracker.remove_client(client_id);
                    let _ = self.resize_tx.send((max_w, max_h));
                }
            }
        }

        let _guard = ClientGuard {
            telemetry: telemetry.clone(),
            resize_tracker: resize_tracker.clone(),
            resize_tx: resize_tx.clone(),
            client_id: client_id.clone(),
        };
        let mut first = true;
        let mut last_frame = 0u64;
        let mut glyph_buffer: Vec<u8> = Vec::new();
        let mut lum_buffer: Vec<u8> = Vec::new();
        let mut packed_buffer: Vec<u8> = Vec::new();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                recv = rx.recv() => {
                    match recv {
                        Ok(frame) => {
                            if frame.stale_for(last_frame) {
                                continue;
                            }
                            if first {
                                first = false;
                                let snapshot = latest.read().await.clone();
                                let first_frame = first_frame_for_client(&snapshot, &frame);
                                yield Ok(datastar_signal_event(
                                    &first_frame,
                                    &mut glyph_buffer,
                                    &mut lum_buffer,
                                    &mut packed_buffer,
                                ));
                            }
                            yield Ok(datastar_signal_event(
                                &frame,
                                &mut glyph_buffer,
                                &mut lum_buffer,
                                &mut packed_buffer,
                            ));
                            last_frame = frame.frame_id;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            telemetry.add_drops(n);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
}

async fn cmd(
    State(state): State<AppState>,
    Path(op): Path<String>,
    body: Option<Json<CmdBody>>,
) -> StatusCode {
    // Command endpoint intentionally returns transport acknowledgement only.
    // Clients observe resulting state changes through /events SSE updates.
    match op.as_str() {
        "toggle_pause" => {
            let _ = state.shared.control_tx.send(ControlMessage::TogglePause);
        }
        "speed_up" => {
            let _ = state.shared.control_tx.send(ControlMessage::Speed(1));
        }
        "speed_down" => {
            let _ = state.shared.control_tx.send(ControlMessage::Speed(-1));
        }
        "reset_speed" => {
            let _ = state.shared.control_tx.send(ControlMessage::ResetSpeed);
        }
        "resize" => {
            if let Some(Json(CmdBody::Resize(vp))) = body {
                if let Some(client_id) = normalize_client_id(vp.client_id) {
                    let (max_w, max_h) = state.resize_tracker.set_client_viewport(
                        &client_id,
                        vp.cols.max(1),
                        vp.rows.max(1),
                    );
                    let _ = state.shared.resize_tx.send((max_w, max_h));
                } else {
                    let _ = state
                        .shared
                        .resize_tx
                        .send((vp.cols.max(1), vp.rows.max(1)));
                }
            }
        }
        "glitch" => {
            if let Some(Json(CmdBody::Glitch(glitch))) = body {
                let latest = state.shared.latest.read().await.clone();
                let (x, y) = clamp_glitch_to_frame(glitch, &latest);
                let _ = state
                    .shared
                    .control_tx
                    .send(ControlMessage::Glitch { x, y });
            }
        }
        _ => {}
    }
    StatusCode::NO_CONTENT
}

#[derive(Debug, serde::Deserialize)]
struct ViewportQuery {
    #[serde(rename = "clientId")]
    client_id: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
}

#[derive(Debug, serde::Deserialize)]
struct ViewportBody {
    #[serde(rename = "clientId")]
    client_id: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum CmdBody {
    Resize(ResizeBody),
    Glitch(GlitchBody),
}

#[derive(Debug, serde::Deserialize)]
struct ResizeBody {
    #[serde(rename = "clientId")]
    client_id: Option<String>,
    cols: u16,
    rows: u16,
}

#[derive(Debug, serde::Deserialize)]
struct GlitchBody {
    x: i32,
    y: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::time::{Duration, timeout};

    type TestSharedState = (
        broadcast::Sender<FrameEvent>,
        Arc<RwLock<FrameEvent>>,
        mpsc::UnboundedSender<(u16, u16)>,
        mpsc::UnboundedSender<ControlMessage>,
    );

    fn test_frame() -> FrameEvent {
        FrameEvent {
            frame_id: 0,
            speed_step: 16,
            width: 4,
            height: 4,
            kind: FrameKind::Keyframe,
            cells: vec![],
        }
    }

    fn test_shared_state() -> TestSharedState {
        let (tx, _rx) = broadcast::channel(8);
        let latest = Arc::new(RwLock::new(test_frame()));
        let (resize_tx, _resize_rx) = mpsc::unbounded_channel();
        let (control_tx, _control_rx) = mpsc::unbounded_channel();
        (tx, latest, resize_tx, control_tx)
    }

    #[test]
    fn random_port_uses_zero_bind() {
        assert_eq!(bind_port(None), 0);
        assert_eq!(bind_port(Some(1234)), 1234);
    }

    #[test]
    fn bind_host_defaults_localhost_and_server_is_unspecified() {
        assert_eq!(bind_host(false), Ipv4Addr::LOCALHOST);
        assert_eq!(bind_host(true), Ipv4Addr::UNSPECIFIED);
    }

    #[test]
    fn late_join_frame_is_keyframe() {
        let snapshot = FrameEvent {
            frame_id: 41,
            speed_step: 16,
            width: 2,
            height: 2,
            kind: FrameKind::Delta,
            cells: vec![],
        };
        let trigger = FrameEvent {
            frame_id: 42,
            speed_step: 16,
            width: 2,
            height: 2,
            kind: FrameKind::Delta,
            cells: vec![],
        };
        let first = first_frame_for_client(&snapshot, &trigger);
        assert_eq!(first.kind, FrameKind::Keyframe);
        assert_eq!(first.frame_id, 42);
    }

    #[test]
    fn browser_markup_contains_disconnect_and_binary_parser_checks() {
        assert!(INDEX_HTML.contains("const SIGNAL_LOST_ART = ["));
        assert!(INDEX_HTML.contains("<<< [ SIGNAL LOST ] >>>"));
        assert!(INDEX_HTML.contains("data-effect=\"window.__matrixDatastar.onFrame($frameId, $speed, $sentMs, $packedB64, $width, $height)\""));
        assert!(INDEX_HTML.contains("data-init=\"@patch('/events'"));
        assert!(INDEX_HTML.contains("datastar.js"));
    }

    #[test]
    fn browser_markup_uses_datastar_post_handlers_for_commands() {
        assert!(INDEX_HTML.contains("data-on:keydown__window=\"if (evt.key === '?') { window.__matrixDatastar.toggleStats(); }"));
        assert!(INDEX_HTML.contains("@post('/cmd/toggle_pause', {contentType: 'form'})"));
        assert!(INDEX_HTML.contains("@post('/cmd/speed_up', {contentType: 'form'})"));
        assert!(INDEX_HTML.contains("@post('/cmd/speed_down', {contentType: 'form'})"));
        assert!(INDEX_HTML.contains("@post('/cmd/reset_speed', {contentType: 'form'})"));
        assert!(INDEX_HTML.contains("@post('/cmd/resize')"));
        assert!(INDEX_HTML.contains("@post('/cmd/glitch')"));
        assert!(!INDEX_HTML.contains("fetch('/cmd/"));
        assert!(!INDEX_HTML.contains("fetch(`/cmd/"));
    }

    #[test]
    fn browser_markup_keeps_cross_browser_fallbacks() {
        assert!(INDEX_HTML.contains(
            "window.crypto && window.crypto.randomUUID ? window.crypto.randomUUID() : ('c-' + Math.random().toString(36).slice(2))",
        ));
        assert!(
            INDEX_HTML.contains(
                "window.visualViewport ? window.visualViewport.height : window.innerHeight",
            )
        );
        assert!(INDEX_HTML.contains("window.devicePixelRatio || 1"));
    }

    #[test]
    fn browser_markup_keeps_transport_agnostic_renderer_wiring() {
        assert!(INDEX_HTML.contains("data-init=\"@patch('/events'"));
        assert!(INDEX_HTML.contains("data-effect=\"window.__matrixDatastar.onFrame($frameId, $speed, $sentMs, $packedB64, $width, $height)\""));
        assert!(INDEX_HTML.contains("@post('/cmd/resize')"));
        assert!(INDEX_HTML.contains("@post('/cmd/glitch')"));
        assert!(!INDEX_HTML.contains("@patch('https://"));
        assert!(!INDEX_HTML.contains("@patch('http://"));
    }

    #[test]
    fn browser_markup_contains_stale_and_disconnect_logic() {
        assert!(INDEX_HTML.contains("if (frameId <= lastRenderedFrame) return;"));
        assert!(INDEX_HTML.contains("renderSignalLost(width, height);"));
        assert!(INDEX_HTML.contains("const margin = 1;"));
    }

    #[test]
    fn glitch_coords_are_clamped_to_latest_frame() {
        let frame = FrameEvent {
            frame_id: 1,
            speed_step: 16,
            width: 10,
            height: 5,
            kind: FrameKind::Keyframe,
            cells: vec![],
        };
        let (x, y) = clamp_glitch_to_frame(GlitchBody { x: -5, y: 500 }, &frame);
        assert_eq!((x, y), (0, 4));
    }

    #[tokio::test]
    async fn tls_mode_fails_fast_for_invalid_cert_paths() {
        let token = CancellationToken::new();
        let telemetry = Arc::new(Telemetry::default());
        let (tx, latest, resize_tx, control_tx) = test_shared_state();

        let result = spawn_server(
            token,
            Some(0),
            false,
            WebTransport::HttpsProvided {
                cert_path: PathBuf::from("/definitely/missing/cert.pem"),
                key_path: PathBuf::from("/definitely/missing/key.pem"),
            },
            tx,
            latest,
            resize_tx,
            control_tx,
            telemetry,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn http_mode_serves_root_and_events() {
        let token = CancellationToken::new();
        let telemetry = Arc::new(Telemetry::default());
        let (tx, latest, resize_tx, control_tx) = test_shared_state();
        let task = spawn_server(
            token.clone(),
            Some(0),
            false,
            WebTransport::Http,
            tx,
            latest,
            resize_tx,
            control_tx,
            telemetry,
        )
        .await
        .expect("http spawn should work");
        assert_eq!(task.scheme, "http");

        let client = reqwest::Client::builder()
            .build()
            .expect("client should build");
        let root = client
            .get(format!("http://127.0.0.1:{}/", task.port))
            .send()
            .await
            .expect("http root should respond");
        assert!(root.status().is_success());

        let events = client
            .get(format!(
                "http://127.0.0.1:{}/events?cols=4&rows=4",
                task.port
            ))
            .send()
            .await
            .expect("http events should respond");
        assert!(events.status().is_success());
        let content_type = events
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.starts_with("text/event-stream"));
        drop(events);

        task.shutdown.cancel();
        let _ = task.handle.await.expect("task join should succeed");
    }

    #[tokio::test]
    async fn cmd_endpoint_acknowledges_without_state_payload() {
        let token = CancellationToken::new();
        let telemetry = Arc::new(Telemetry::default());
        let (tx, latest, resize_tx, control_tx) = test_shared_state();
        let task = spawn_server(
            token.clone(),
            Some(0),
            false,
            WebTransport::Http,
            tx,
            latest,
            resize_tx,
            control_tx,
            telemetry,
        )
        .await
        .expect("http spawn should work");

        let client = reqwest::Client::builder()
            .build()
            .expect("client should build");

        let supported = client
            .post(format!("http://127.0.0.1:{}/cmd/speed_up", task.port))
            .send()
            .await
            .expect("supported command should respond");
        assert_eq!(supported.status(), StatusCode::NO_CONTENT);
        let supported_body = supported.bytes().await.expect("supported body should read");
        assert!(supported_body.is_empty());

        let unknown = client
            .post(format!("http://127.0.0.1:{}/cmd/not-a-real-op", task.port))
            .send()
            .await
            .expect("unknown command should respond");
        assert_eq!(unknown.status(), StatusCode::NO_CONTENT);
        let unknown_body = unknown.bytes().await.expect("unknown body should read");
        assert!(unknown_body.is_empty());

        let events = client
            .get(format!(
                "http://127.0.0.1:{}/events?cols=4&rows=4",
                task.port
            ))
            .send()
            .await
            .expect("events should remain available after commands");
        assert!(events.status().is_success());
        let content_type = events
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.starts_with("text/event-stream"));
        drop(events);

        task.shutdown.cancel();
        let _ = task.handle.await.expect("task join should succeed");
    }

    #[tokio::test]
    async fn rapid_commands_are_acked_and_enqueued_in_order() {
        let (tx, _rx) = broadcast::channel(8);
        let latest = Arc::new(RwLock::new(test_frame()));
        let (resize_tx, _resize_rx) = mpsc::unbounded_channel();
        let (control_tx, mut control_rx) = mpsc::unbounded_channel();
        let state = AppState {
            shared: Arc::new(SharedStreamState {
                tx,
                latest,
                resize_tx,
                control_tx,
            }),
            telemetry: Arc::new(Telemetry::default()),
            shutdown: CancellationToken::new(),
            resize_tracker: Arc::new(ResizeTracker::default()),
        };

        let a = cmd(State(state.clone()), Path("speed_up".to_string()), None).await;
        let b = cmd(State(state.clone()), Path("speed_down".to_string()), None).await;
        let c = cmd(State(state.clone()), Path("reset_speed".to_string()), None).await;

        assert_eq!(a, StatusCode::NO_CONTENT);
        assert_eq!(b, StatusCode::NO_CONTENT);
        assert_eq!(c, StatusCode::NO_CONTENT);

        let m1 = control_rx.recv().await.expect("first command must enqueue");
        let m2 = control_rx
            .recv()
            .await
            .expect("second command must enqueue");
        let m3 = control_rx.recv().await.expect("third command must enqueue");
        assert!(matches!(m1, ControlMessage::Speed(1)));
        assert!(matches!(m2, ControlMessage::Speed(-1)));
        assert!(matches!(m3, ControlMessage::ResetSpeed));

        let u = cmd(State(state), Path("not-a-real-op".to_string()), None).await;
        assert_eq!(u, StatusCode::NO_CONTENT);
        let extra = timeout(Duration::from_millis(20), control_rx.recv()).await;
        assert!(
            !matches!(extra, Ok(Some(_))),
            "unknown command must not enqueue control messages"
        );
    }

    #[tokio::test]
    async fn https_mode_serves_root_and_events_with_brotli() {
        let token = CancellationToken::new();
        let telemetry = Arc::new(Telemetry::default());
        let (tx, latest, resize_tx, control_tx) = test_shared_state();
        let task = spawn_server(
            token.clone(),
            Some(0),
            false,
            WebTransport::HttpsAuto,
            tx,
            latest,
            resize_tx,
            control_tx,
            telemetry,
        )
        .await
        .expect("https spawn should work");
        assert_eq!(task.scheme, "https");

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("client should build");
        let root = client
            .get(format!("https://127.0.0.1:{}/", task.port))
            .send()
            .await
            .expect("https root should respond");
        assert!(root.status().is_success());

        let events = client
            .get(format!(
                "https://127.0.0.1:{}/events?cols=4&rows=4",
                task.port
            ))
            .header("accept-encoding", "br, gzip")
            .send()
            .await
            .expect("https events should respond");
        assert!(events.status().is_success());
        let content_type = events
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.starts_with("text/event-stream"));
        let content_encoding = events
            .headers()
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_encoding, "br");
        drop(events);

        task.shutdown.cancel();
        let _ = task.handle.await.expect("task join should succeed");
    }
}
