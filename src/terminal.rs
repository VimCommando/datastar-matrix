use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use crossterm::ExecutableCommand;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

use crate::ControlMessage;
use crate::frame::{FrameEvent, FrameKind};
use crate::telemetry::Telemetry;

pub fn initial_terminal_size() -> (u16, u16) {
    current_terminal_size().unwrap_or((120, 40))
}

pub fn current_terminal_size() -> Option<(u16, u16)> {
    size().ok().map(|(w, h)| (w.max(1), h.max(1)))
}

pub fn run_terminal(
    mut rx: broadcast::Receiver<FrameEvent>,
    token: CancellationToken,
    telemetry: Arc<Telemetry>,
    control_tx: mpsc::UnboundedSender<ControlMessage>,
) -> anyhow::Result<()> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("failed to enter alternate screen")?;
    stdout
        .execute(EnableMouseCapture)
        .context("failed to enable mouse capture")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal")?;

    let result = run_loop(&mut terminal, &mut rx, &token, &telemetry, &control_tx);

    disable_raw_mode().ok();
    terminal.backend_mut().execute(DisableMouseCapture).ok();
    terminal.backend_mut().execute(LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    rx: &mut broadcast::Receiver<FrameEvent>,
    token: &CancellationToken,
    telemetry: &Telemetry,
    control_tx: &mpsc::UnboundedSender<ControlMessage>,
) -> anyhow::Result<()> {
    let mut show_overlay = false;
    let mut buffer: Vec<char> = Vec::new();
    let mut lum_buffer: Vec<u8> = Vec::new();
    let mut width = 0;
    let mut height = 0;
    let mut current_text = String::new();
    let mut last_frame = 0;
    let mut fps = 0.0f32;
    let mut fps_frame = 0u64;
    let mut fps_at = Instant::now();
    let mut current_speed: u8 = 16;

    while !token.is_cancelled() {
        loop {
            match rx.try_recv() {
                Ok(frame) => {
                    if frame.stale_for(last_frame) {
                        continue;
                    }
                    if width != frame.width || height != frame.height {
                        width = frame.width;
                        height = frame.height;
                    }
                    if frame.kind == FrameKind::Keyframe {
                        buffer.clear();
                        lum_buffer.clear();
                    }
                    current_text = frame.as_text(&mut buffer);
                    apply_luminance(&frame, &mut lum_buffer);
                    last_frame = frame.frame_id;
                    current_speed = frame.speed_step;
                    if fps_at.elapsed() >= Duration::from_millis(400) {
                        let elapsed = fps_at.elapsed().as_secs_f32().max(0.001);
                        let delta = frame.frame_id.saturating_sub(fps_frame) as f32;
                        fps = delta / elapsed;
                        fps_frame = frame.frame_id;
                        fps_at = Instant::now();
                    }
                }
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    telemetry.add_drops(n as u64);
                    continue;
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                Err(broadcast::error::TryRecvError::Closed) => return Ok(()),
            }
        }

        terminal.draw(|f| {
            let area = f.area();
            let lines = build_gradient_lines(&buffer, &lum_buffer, width, height);
            let paragraph = if lines.is_empty() {
                Paragraph::new(Text::raw(current_text.as_str()))
            } else {
                Paragraph::new(Text::from(lines))
            };
            f.render_widget(paragraph, area);

            if show_overlay {
                let (clients, frames, _drops) = telemetry.snapshot();
                let text = format!(
                    "[ clients:{clients}  speed:{}  frame:{frames}  fps:{:.1} ]",
                    current_speed, fps
                );
                let y = area.y + area.height.saturating_sub(1);
                let text_width = text.chars().count() as u16;
                let overlay_x = area.x + area.width.saturating_sub(text_width);
                let line = Line::from(vec![Span::styled(
                    text,
                    Style::default().fg(Color::Rgb(180, 255, 180)),
                )]);
                f.render_widget(
                    Paragraph::new(line),
                    ratatui::layout::Rect {
                        x: overlay_x,
                        y,
                        width: text_width.min(area.width),
                        height: 1,
                    },
                );
            }
        })?;

        if event::poll(Duration::from_millis(16)).context("event poll failed")? {
            match event::read().context("event read failed")? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('?') => show_overlay = !show_overlay,
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('+') => {
                        let _ = control_tx.send(ControlMessage::Speed(1));
                    }
                    KeyCode::Char('-') => {
                        let _ = control_tx.send(ControlMessage::Speed(-1));
                    }
                    KeyCode::Char('0') => {
                        let _ = control_tx.send(ControlMessage::ResetSpeed);
                    }
                    KeyCode::Char(' ') => {
                        let _ = control_tx.send(ControlMessage::TogglePause);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                        let _ = control_tx.send(ControlMessage::Glitch {
                            x: mouse.column,
                            y: mouse.row,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn apply_luminance(frame: &FrameEvent, lum_buffer: &mut Vec<u8>) {
    let needed = frame.width as usize * frame.height as usize;
    if lum_buffer.len() != needed {
        *lum_buffer = vec![0; needed];
    }
    if frame.kind == FrameKind::Keyframe {
        lum_buffer.fill(0);
    }
    for cell in &frame.cells {
        if cell.x < frame.width && cell.y < frame.height {
            let idx = cell.y as usize * frame.width as usize + cell.x as usize;
            if idx < lum_buffer.len() {
                lum_buffer[idx] = cell.lum;
            }
        }
    }
}

fn build_gradient_lines(
    grid: &[char],
    lum_buffer: &[u8],
    width: u16,
    height: u16,
) -> Vec<Line<'static>> {
    if width == 0 || height == 0 || grid.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::with_capacity(height as usize);
    for y in 0..height as usize {
        let mut spans = Vec::with_capacity(width as usize);
        for x in 0..width as usize {
            let idx = y * width as usize + x;
            if idx >= grid.len() {
                continue;
            }
            let ch = grid[idx];
            let green = lum_buffer.get(idx).copied().unwrap_or(0);
            let fg = if green >= 250 {
                Color::Rgb(180, 255, 180)
            } else {
                Color::Rgb(0, green, 0)
            };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(fg)));
        }
        lines.push(Line::from(spans));
    }
    lines
}
