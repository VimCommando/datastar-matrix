pub mod config;
pub mod frame;
pub mod glyph;
pub mod simulation;
pub mod telemetry;
pub mod terminal;
#[cfg(feature = "web")]
pub mod web;

use std::sync::Arc;

use anyhow::Context;
use frame::FrameEvent;
use simulation::Simulation;
use telemetry::Telemetry;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant, interval};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy)]
pub enum ControlMessage {
    TogglePause,
    Speed(i8),
    ResetSpeed,
}

pub struct SharedStreamState {
    pub tx: broadcast::Sender<FrameEvent>,
    pub latest: Arc<RwLock<FrameEvent>>,
    pub resize_tx: mpsc::UnboundedSender<(u16, u16)>,
    pub control_tx: mpsc::UnboundedSender<ControlMessage>,
}

pub async fn run() -> anyhow::Result<()> {
    let cfg = config::Config::parse();
    let token = CancellationToken::new();
    let telemetry = Arc::new(Telemetry::default());

    let (width, height) = terminal::initial_terminal_size();
    let simulation = Simulation::new(width, height, cfg.target_fps);
    let mut initial_frame = simulation.keyframe();
    initial_frame.speed_step = 16;

    let (tx, _rx) = broadcast::channel(256);
    let (resize_tx, resize_rx) = mpsc::unbounded_channel();
    let (control_tx, control_rx) = mpsc::unbounded_channel();
    let shared = SharedStreamState {
        tx,
        latest: Arc::new(RwLock::new(initial_frame)),
        resize_tx: resize_tx.clone(),
        control_tx: control_tx.clone(),
    };

    let simulation_task = spawn_simulation_task(
        token.clone(),
        shared.tx.clone(),
        shared.latest.clone(),
        telemetry.clone(),
        simulation,
        resize_rx,
        control_rx,
    );

    #[cfg(feature = "web")]
    let web_task = if cfg.web_enabled() {
        Some(web::spawn_server(
            token.clone(),
            cfg.port,
            cfg.server,
            shared.tx.clone(),
            shared.latest.clone(),
            shared.resize_tx.clone(),
            shared.control_tx.clone(),
            telemetry.clone(),
        )?)
    } else {
        None
    };

    #[cfg(not(feature = "web"))]
    let _ = cfg;

    let terminal_task = tokio::task::spawn_blocking({
        let rx = shared.tx.subscribe();
        let token = token.clone();
        let telemetry = telemetry.clone();
        let control_tx = control_tx.clone();
        move || terminal::run_terminal(rx, token, telemetry, control_tx)
    });

    let result = supervise(
        token.clone(),
        simulation_task,
        terminal_task,
        #[cfg(feature = "web")]
        web_task,
    )
    .await;

    token.cancel();
    result
}

fn spawn_simulation_task(
    token: CancellationToken,
    tx: broadcast::Sender<FrameEvent>,
    latest: Arc<RwLock<FrameEvent>>,
    telemetry: Arc<Telemetry>,
    mut simulation: Simulation,
    mut resize_rx: mpsc::UnboundedReceiver<(u16, u16)>,
    mut control_rx: mpsc::UnboundedReceiver<ControlMessage>,
) -> JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move {
        let base_fps = simulation.target_fps().max(1.0);
        let mut speed_step: i32 = 16;
        let mut paused = false;
        let mut ticker = interval(tick_duration(base_fps, speed_step));
        let mut last_resize_check = Instant::now();
        let mut last_heartbeat_at = Instant::now();

        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                Some((w, h)) = resize_rx.recv() => {
                    simulation.grow_to_fit(w, h);
                }
                Some(control) = control_rx.recv() => {
                    match control {
                        ControlMessage::TogglePause => {
                            paused = !paused;
                            if paused {
                                last_heartbeat_at = Instant::now() - Duration::from_secs(1);
                            }
                        }
                        ControlMessage::Speed(delta) => {
                            speed_step = (speed_step + delta as i32).clamp(1, 32);
                            ticker = interval(tick_duration(base_fps, speed_step));
                        }
                        ControlMessage::ResetSpeed => {
                            speed_step = 16;
                            ticker = interval(tick_duration(base_fps, speed_step));
                        }
                    }
                }
                _ = ticker.tick() => {
                    if paused {
                        if last_heartbeat_at.elapsed() >= Duration::from_secs(1) {
                            let heartbeat = latest.read().await.clone();
                            let _ = tx.send(heartbeat);
                            last_heartbeat_at = Instant::now();
                        }
                        continue;
                    }
                    if last_resize_check.elapsed() > Duration::from_millis(100) {
                        if let Some((w, h)) = terminal::current_terminal_size() {
                            simulation.grow_to_fit(w, h);
                        }
                        last_resize_check = Instant::now();
                    }

                    let mut frame = simulation.tick();
                    frame.speed_step = speed_step as u8;
                    telemetry.increment_frames();
                    {
                        let mut latest_lock = latest.write().await;
                        *latest_lock = frame.clone();
                    }
                    let _ = tx.send(frame);
                }
            }
        }

        Ok(())
    })
}

fn tick_duration(base_fps: f32, speed_step: i32) -> Duration {
    let factor = (speed_step as f32 / 16.0).max(1.0 / 16.0);
    let effective_fps = (base_fps * factor).max(1.0);
    Duration::from_secs_f64(1.0 / effective_fps as f64)
}

async fn supervise(
    token: CancellationToken,
    simulation_task: JoinHandle<anyhow::Result<()>>,
    terminal_task: JoinHandle<anyhow::Result<()>>,
    #[cfg(feature = "web")] web_task: Option<web::WebTask>,
) -> anyhow::Result<()> {
    #[cfg(feature = "web")]
    {
        if let Some(web_task) = web_task {
            tokio::select! {
                res = simulation_task => {
                    token.cancel();
                    res.context("simulation task panicked")??;
                    Ok(())
                }
                res = terminal_task => {
                    token.cancel();
                    web_task.shutdown.cancel();
                    res.context("terminal task panicked")??;
                    Err(anyhow::anyhow!("terminal exited"))
                }
                res = web_task.handle => {
                    token.cancel();
                    res.context("web task panicked")??;
                    Err(anyhow::anyhow!("web server exited"))
                }
            }
        } else {
            tokio::select! {
                res = simulation_task => {
                    token.cancel();
                    res.context("simulation task panicked")??;
                    Ok(())
                }
                res = terminal_task => {
                    token.cancel();
                    res.context("terminal task panicked")??;
                    Err(anyhow::anyhow!("terminal exited"))
                }
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
        tokio::select! {
            res = simulation_task => {
                token.cancel();
                res.context("simulation task panicked")??;
                Ok(())
            }
            res = terminal_task => {
                token.cancel();
                res.context("terminal task panicked")??;
                Err(anyhow::anyhow!("terminal exited"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn coupled_shutdown_returns_error_when_terminal_fails() {
        let token = CancellationToken::new();
        let sim = tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            Ok::<(), anyhow::Error>(())
        });
        let terminal = tokio::spawn(async { Err::<(), anyhow::Error>(anyhow::anyhow!("boom")) });

        #[cfg(feature = "web")]
        let result = supervise(token, sim, terminal, None).await;
        #[cfg(not(feature = "web"))]
        let result = supervise(token, sim, terminal).await;

        assert!(result.is_err());
    }
}
