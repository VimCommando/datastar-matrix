use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_stream::stream;
use axum::extract::{Path, Query, State};
use axum::Json;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use datastar::prelude::PatchSignals;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::compression::CompressionLayer;

use crate::SharedStreamState;
use crate::frame::{FrameEvent, FrameKind};
use crate::telemetry::Telemetry;
use crate::ControlMessage;

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
      --disc: #d8d8d8;
      --g0: rgb(160, 255, 160);
      --g1: rgb(0, 195, 0);
      --g2: rgb(0, 170, 0);
      --g3: rgb(0, 145, 0);
      --g4: rgb(0, 95, 0);
      --g5: rgb(0, 20, 0);
    }
    body { margin: 0; background: var(--bg); color: var(--g1); font-family: monospace; }
    .wrap { position: relative; width: 100vw; height: 100vh; overflow: hidden; }
    #matrix {
      margin: 0;
      display: block;
      width: 100%;
      height: 100%;
    }
    #disc { position: absolute; inset: 0; display: none; align-items: center; justify-content: center; color: var(--disc); font-size: 22px; }
    #stats {
      position: absolute;
      left: 0;
      right: 0;
      bottom: 0;
      display: none;
      padding: 2px 8px;
      color: #d0d0d0;
      background: rgba(0, 0, 0, 0.45);
      font-size: 13px;
      line-height: 1.1;
      white-space: nowrap;
      text-align: right;
    }
  </style>
</head>
<body>
  <div class="wrap">
    <canvas id="matrix" data-on:mousedown="window.__matrixDatastar.onClick(evt)"></canvas>
    <div id="stats"></div>
    <div id="disc">[ Disconnected ]</div>
    <div
      id="ds"
      data-signals="{frameId: 0, speed: 16, sentMs: 0, connected: false, packedB64: '', cols: Math.max(1, Math.ceil(window.innerWidth / 10)), rows: Math.max(1, Math.ceil(window.innerHeight / 20)), width: Math.max(1, Math.ceil(window.innerWidth / 10)), height: Math.max(1, Math.ceil(window.innerHeight / 20))}"
      data-init="@patch('/events', {cols: $cols, rows: $rows})"
      data-effect="window.__matrixDatastar.onFrame($frameId, $speed, $sentMs, $packedB64, $width, $height)"
      data-on:keydown__window="window.__matrixDatastar.onKey(evt)"
      data-on:resize__window__debounce.500ms="$cols = Math.max(1, Math.ceil(window.innerWidth / 10)); $rows = Math.max(1, Math.ceil(window.innerHeight / 20)); $width = $cols; $height = $rows; window.__matrixDatastar.onResize($cols, $rows)"
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
    const stats = document.getElementById('stats');
    const disc = document.getElementById('disc');
    let timeoutHandle;
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

    function scheduleDisconnect() {
      clearTimeout(timeoutHandle);
      timeoutHandle = setTimeout(() => { disc.style.display = 'flex'; }, 3000);
    }

    function renderStats(frameId, speed, sentMs) {
      if (showStats) {
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
        stats.style.display = 'block';
        stats.textContent = `frames:${frameId}  fps:${fps.toFixed(1)}  latency:${latencyMs.toFixed(1)}ms  speed:${speed}`;
      } else {
        stats.style.display = 'none';
      }
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

    function setupCanvas(width, height) {
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
      ctx.fillStyle = '#000';
      ctx.fillRect(0, 0, width * CELL_W, height * CELL_H);
    }

    function renderCanvas(packedB64, width, height, frameId) {
      if (!ctx || !packedB64 || !width || !height) {
        return;
      }
      if (frameId <= lastRenderedFrame) return;
      setupCanvas(width, height);
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
        renderCanvas(packedB64 || '', width, height, frameId);
        scheduleDisconnect();
        disc.style.display = 'none';
        renderStats(frameId, speed, sentMs);
        lastFrameId = frameId;
        lastSpeed = speed;
        lastSentMs = sentMs;
      },
      onKey(evt) {
        const sendControl = (op) => {
          fetch(`/cmd/${op}`, { method: 'POST' }).catch(() => {});
        };
        if (evt.key === '?') {
          showStats = !showStats;
          renderStats(lastFrameId, lastSpeed, lastSentMs);
        } else if (evt.key === ' ') {
          evt.preventDefault();
          sendControl('toggle_pause');
        } else if (evt.key === '+' || evt.key === '=') {
          sendControl('speed_up');
        } else if (evt.key === '-' || evt.key === '_') {
          sendControl('speed_down');
        } else if (evt.key === '0') {
          sendControl('reset_speed');
        }
      },
      onResize(cols, rows) {
        cols = Number(cols || 1);
        rows = Number(rows || 1);
        if (cols === lastResizeCols && rows === lastResizeRows) return;
        lastResizeCols = cols;
        lastResizeRows = rows;
        fetch('/cmd/resize', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ cols, rows }),
        }).catch(() => {});
      },
      onClick(evt) {
        const rect = matrix.getBoundingClientRect();
        const px = evt.clientX - rect.left;
        const py = evt.clientY - rect.top;
        let x = Math.floor(px / CELL_W);
        let y = Math.floor(py / CELL_H);
        const maxX = Math.max(0, canvasW - 1);
        const maxY = Math.max(0, canvasH - 1);
        x = Math.max(0, Math.min(maxX, x));
        y = Math.max(0, Math.min(maxY, y));
        fetch('/cmd/glitch', {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ x, y }),
        }).catch(() => {});
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
}

pub struct WebTask {
    pub handle: JoinHandle<anyhow::Result<()>>,
    pub shutdown: CancellationToken,
    pub port: u16,
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

pub fn spawn_server(
    token: CancellationToken,
    requested_port: Option<u16>,
    public_server: bool,
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
    let listener = TcpListener::from_std(std_listener).context("failed to create tokio listener")?;

    let app_state = AppState {
        shared: Arc::new(SharedStreamState {
            tx,
            latest,
            resize_tx,
            control_tx,
        }),
        telemetry,
        shutdown: shutdown.clone(),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/events", get(events_datastar).patch(events_datastar_patch))
        .route("/cmd/{op}", post(cmd))
        .with_state(app_state)
        .layer(CompressionLayer::new());

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
            .context("web serve failed")?;
        Ok(())
    });

    Ok(WebTask {
        handle,
        shutdown: token,
        port,
    })
}

async fn index() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn events_datastar(
    State(state): State<AppState>,
    Query(viewport): Query<ViewportQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    events_datastar_inner(state, viewport.cols, viewport.rows)
}

async fn events_datastar_patch(
    State(state): State<AppState>,
    Json(viewport): Json<ViewportBody>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    events_datastar_inner(state, viewport.cols, viewport.rows)
}

fn events_datastar_inner(
    state: AppState,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    if let (Some(cols), Some(rows)) = (cols, rows) {
        let _ = state.shared.resize_tx.send((cols.max(1), rows.max(1)));
    }
    state.telemetry.inc_clients();
    let mut rx = state.shared.tx.subscribe();
    let latest = state.shared.latest.clone();
    let telemetry = state.telemetry.clone();
    let shutdown = state.shutdown.clone();

    let stream = stream! {
        struct ClientGuard(Arc<Telemetry>);
        impl Drop for ClientGuard {
            fn drop(&mut self) {
                self.0.dec_clients();
            }
        }

        let _guard = ClientGuard(telemetry.clone());
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
                            telemetry.add_drops(n as u64);
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
                let _ = state.shared.resize_tx.send((vp.cols.max(1), vp.rows.max(1)));
            }
        }
        "glitch" => {
            if let Some(Json(CmdBody::Glitch(glitch))) = body {
                let latest = state.shared.latest.read().await.clone();
                let (x, y) = clamp_glitch_to_frame(glitch, &latest);
                let _ = state.shared.control_tx.send(ControlMessage::Glitch { x, y });
            }
        }
        _ => {}
    }
    StatusCode::NO_CONTENT
}

#[derive(Debug, serde::Deserialize)]
struct ViewportQuery {
    cols: Option<u16>,
    rows: Option<u16>,
}

#[derive(Debug, serde::Deserialize)]
struct ViewportBody {
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
        assert!(INDEX_HTML.contains("[ Disconnected ]"));
        assert!(INDEX_HTML.contains("data-effect=\"window.__matrixDatastar.onFrame($frameId, $speed, $sentMs, $packedB64, $width, $height)\""));
        assert!(INDEX_HTML.contains("data-init=\"@patch('/events'"));
        assert!(INDEX_HTML.contains("datastar.js"));
    }

    #[test]
    fn browser_markup_contains_stale_and_disconnect_logic() {
        assert!(INDEX_HTML.contains("if (frameId <= lastRenderedFrame) return;"));
        assert!(INDEX_HTML.contains("setTimeout(() => { disc.style.display = 'flex'; }, 3000)"));
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
}
