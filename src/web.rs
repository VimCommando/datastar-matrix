use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_stream::stream;
use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

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
      white-space: pre;
      font-size: 12pt;
      line-height: 1;
      font-variant-ligatures: none;
      font-feature-settings: "liga" 0, "calt" 0;
      letter-spacing: 0;
    }
    .cell {
      display: inline-block;
      width: 1ch;
      text-align: center;
      overflow: hidden;
      vertical-align: top;
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
    .g0 { color: var(--g0); text-shadow: 0 0 6px rgba(0, 255, 0, 0.45); }
    .g1 { color: var(--g1); }
    .g2 { color: var(--g2); }
    .g3 { color: var(--g3); }
    .g4 { color: var(--g4); }
    .g5 { color: var(--g5); }
  </style>
</head>
<body>
  <div class="wrap">
    <pre id="matrix"></pre>
    <div id="stats"></div>
    <div id="disc">[ Disconnected ]</div>
  </div>
  <script>
    const signals = { frameId: 0, speed: 16, w: 0, h: 0, buf: [], lum: [] };
    const glyphs = [
      ' ','0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z',
      'ｱ','ｲ','ｳ','ｴ','ｵ','ｶ','ｷ','ｸ','ｹ','ｺ','ｻ','ｼ','ｽ','ｾ','ｿ','ﾀ','ﾁ','ﾂ','ﾃ','ﾄ','ﾅ','ﾆ','ﾇ','ﾈ','ﾉ','ﾊ','ﾋ','ﾌ','ﾍ','ﾎ','ﾏ','ﾐ','ﾑ','ﾒ','ﾓ','ﾔ','ﾕ','ﾖ','ﾗ','ﾘ','ﾙ','ﾚ','ﾛ','ﾜ','ﾝ'
    ];
    const matrix = document.getElementById('matrix');
    const stats = document.getElementById('stats');
    const disc = document.getElementById('disc');
    let timeoutHandle;
    let showStats = false;
    let fps = 0;
    let fpsLastFrame = 0;
    let fpsLastAt = performance.now();
    let latencySamples = [];
    let latencyMs = 0;

    function scheduleDisconnect() {
      clearTimeout(timeoutHandle);
      timeoutHandle = setTimeout(() => { disc.style.display = 'flex'; }, 3000);
    }

    function escapeHtml(ch) {
      if (ch === '&') return '&amp;';
      if (ch === '<') return '&lt;';
      if (ch === '>') return '&gt;';
      return ch;
    }

    function classFromLum(lum) {
      if (lum >= 235) return 0;
      if (lum >= 180) return 1;
      if (lum >= 150) return 2;
      if (lum >= 120) return 3;
      if (lum >= 80) return 4;
      return 5;
    }

    function readU64(dv, p) {
      let out = 0n;
      for (let i = 0; i < 8; i++) {
        out |= BigInt(dv.getUint8(p + i)) << (8n * BigInt(i));
      }
      return out;
    }

    function render() {
      let html = '';
      for (let y = 0; y < signals.h; y++) {
        for (let x = 0; x < signals.w; x++) {
          const idx = y * signals.w + x;
          const ch = signals.buf[idx];
          if (ch === ' ') {
            html += ' ';
            continue;
          }
          const lum = signals.lum[idx] || 0;
          html += `<span class="cell g${classFromLum(lum)}">${escapeHtml(ch)}</span>`;
        }
        if (y + 1 < signals.h) html += '\n';
      }
      matrix.innerHTML = html;
      if (showStats) {
        stats.style.display = 'block';
        stats.textContent = `frames:${signals.frameId}  fps:${fps.toFixed(1)}  latency:${latencyMs.toFixed(1)}ms  speed:${signals.speed}`;
      } else {
        stats.style.display = 'none';
      }
    }

    const cw = Math.max(1, Math.floor(window.innerWidth / 8));
    const ch = Math.max(1, Math.floor(window.innerHeight / 14));
    const es = new EventSource(`/events/matrix?cols=${cw}&rows=${ch}`);
    es.onopen = () => { disc.style.display = 'none'; scheduleDisconnect(); };
    es.onmessage = (msg) => {
      scheduleDisconnect();
      const bin = atob(msg.data);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const dv = new DataView(bytes.buffer);

      let p = 0;
      if (bytes.length < 24) return;
      const frameId = Number(readU64(dv, p)); p += 8;
      const sentMs = Number(readU64(dv, p)); p += 8;
      const speed = dv.getUint8(p); p += 1;
      const width = dv.getUint16(p, true); p += 2;
      const height = dv.getUint16(p, true); p += 2;
      const kind = dv.getUint8(p); p += 1;
      const colCount = dv.getUint16(p, true); p += 2;

      if (frameId <= signals.frameId) return;
      if (kind === 0 || signals.w !== width || signals.h !== height) {
        signals.w = width;
        signals.h = height;
        signals.buf = new Array(width * height).fill(' ');
        signals.lum = new Array(width * height).fill(0);
      }

      for (let c = 0; c < colCount; c++) {
        if (p + 6 > bytes.length) break;
        const x = dv.getUint16(p, true); p += 2;
        const yStart = dv.getUint16(p, true); p += 2;
        const len = dv.getUint16(p, true); p += 2;
        if (p + (len * 2) > bytes.length) break;
        if (x >= signals.w || yStart >= signals.h || (yStart + len) > signals.h) {
          p += len * 2;
          continue;
        }
        for (let i = 0; i < len; i++) {
          const glyph = dv.getUint8(p); p += 1;
          const lum = dv.getUint8(p); p += 1;
          const y = yStart + i;
          const idx = y * signals.w + x;
          if (idx >= 0 && idx < signals.buf.length) {
            signals.buf[idx] = glyphs[glyph] || ' ';
            signals.lum[idx] = lum;
          }
        }
      }

      signals.frameId = frameId;
      signals.speed = speed;
      const now = performance.now();
      if (now - fpsLastAt >= 400) {
        fps = (frameId - fpsLastFrame) / ((now - fpsLastAt) / 1000.0);
        fpsLastFrame = frameId;
        fpsLastAt = now;
      }
      const nowWall = Date.now();
      const oneWay = Math.max(0, nowWall - sentMs);
      latencySamples.push({ t: nowWall, v: oneWay });
      while (latencySamples.length && (nowWall - latencySamples[0].t) > 250) latencySamples.shift();
      if (latencySamples.length) {
        const total = latencySamples.reduce((acc, s) => acc + s.v, 0);
        latencyMs = total / latencySamples.length;
      }
      disc.style.display = 'none';
      render();
    };
    es.onerror = () => { disc.style.display = 'flex'; };
    window.addEventListener('keydown', (ev) => {
      const sendControl = (op) => {
        fetch(`/control?op=${op}`, { method: 'POST' }).catch(() => {});
      };
      if (ev.key === '?') {
        showStats = !showStats;
        render();
      } else if (ev.key === ' ') {
        ev.preventDefault();
        sendControl('toggle_pause');
      } else if (ev.key === '+' || ev.key === '=') {
        sendControl('speed_up');
      } else if (ev.key === '-' || ev.key === '_') {
        sendControl('speed_down');
      } else if (ev.key === '0') {
        sendControl('reset_speed');
      }
    });
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

fn encode_frame(frame: &FrameEvent) -> String {
    let mut cells = frame.cells.clone();
    cells.sort_by_key(|c| (c.x, c.y));

    let mut columns: Vec<(u16, u16, Vec<(u8, u8)>)> = Vec::new();
    let mut i = 0usize;
    while i < cells.len() {
        let x = cells[i].x;
        let start_y = cells[i].y;
        let mut cur_y = start_y;
        let mut payload = vec![(cells[i].glyph, cells[i].lum)];
        i += 1;
        while i < cells.len() && cells[i].x == x && cells[i].y == cur_y + 1 {
            cur_y = cells[i].y;
            payload.push((cells[i].glyph, cells[i].lum));
            i += 1;
        }
        columns.push((x, start_y, payload));
    }

    let mut out = Vec::with_capacity(25 + columns.len() * 6 + cells.len() * 2);
    let sent_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    out.extend_from_slice(&frame.frame_id.to_le_bytes());
    out.extend_from_slice(&sent_ms.to_le_bytes());
    out.push(frame.speed_step);
    out.extend_from_slice(&frame.width.to_le_bytes());
    out.extend_from_slice(&frame.height.to_le_bytes());
    out.push(match frame.kind {
        FrameKind::Keyframe => 0,
        FrameKind::Delta => 1,
    });
    out.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    for (x, y_start, payload) in columns {
        out.extend_from_slice(&x.to_le_bytes());
        out.extend_from_slice(&y_start.to_le_bytes());
        out.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        for (glyph, lum) in payload {
            out.push(glyph);
            out.push(lum);
        }
    }

    STANDARD.encode(out)
}

pub fn spawn_server(
    token: CancellationToken,
    requested_port: Option<u16>,
    tx: broadcast::Sender<FrameEvent>,
    latest: Arc<RwLock<FrameEvent>>,
    resize_tx: mpsc::UnboundedSender<(u16, u16)>,
    control_tx: mpsc::UnboundedSender<ControlMessage>,
    telemetry: Arc<Telemetry>,
) -> anyhow::Result<WebTask> {
    let shutdown = token.child_token();

    let bind_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, bind_port(requested_port)));
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
        .route("/events/matrix", get(events))
        .route("/control", post(control))
        .with_state(app_state);

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

async fn events(
    State(state): State<AppState>,
    Query(viewport): Query<ViewportQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    if let (Some(cols), Some(rows)) = (viewport.cols, viewport.rows) {
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

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                recv = rx.recv() => {
                    match recv {
                        Ok(frame) => {
                            if first {
                                first = false;
                                let snapshot = latest.read().await.clone();
                                let first_frame = first_frame_for_client(&snapshot, &frame);
                                yield Ok(Event::default().data(encode_frame(&first_frame)));
                            }
                            yield Ok(Event::default().data(encode_frame(&frame)));
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

async fn control(
    State(state): State<AppState>,
    Query(control): Query<ControlQuery>,
) -> StatusCode {
    match control.op.as_str() {
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
struct ControlQuery {
    op: String,
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
        assert!(INDEX_HTML.contains("frameId <= signals.frameId"));
        assert!(INDEX_HTML.contains("const colCount = dv.getUint16"));
    }

    #[test]
    fn encodes_binary_frame_with_header_and_columns() {
        let frame = FrameEvent {
            frame_id: 7,
            speed_step: 16,
            width: 3,
            height: 3,
            kind: FrameKind::Delta,
            cells: vec![
                crate::frame::CellUpdate { x: 1, y: 1, glyph: 2, lum: 200 },
                crate::frame::CellUpdate { x: 1, y: 2, glyph: 3, lum: 120 },
            ],
        };
        let encoded = encode_frame(&frame);
        let bytes = STANDARD.decode(encoded).expect("valid base64");
        assert_eq!(u64::from_le_bytes(bytes[0..8].try_into().expect("u64 bytes")), 7);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().expect("cols bytes")), 1);
    }
}
