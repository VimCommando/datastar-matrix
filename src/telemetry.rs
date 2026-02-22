use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct Telemetry {
    clients: AtomicUsize,
    frames: AtomicU64,
    drops: AtomicU64,
}

impl Telemetry {
    pub fn inc_clients(&self) {
        self.clients.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_clients(&self) {
        self.clients.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn increment_frames(&self) {
        self.frames.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_drops(&self, dropped: u64) {
        self.drops.fetch_add(dropped, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (usize, u64, u64) {
        (
            self.clients.load(Ordering::Relaxed),
            self.frames.load(Ordering::Relaxed),
            self.drops.load(Ordering::Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_clients_frames_and_drops() {
        let t = Telemetry::default();
        t.inc_clients();
        t.increment_frames();
        t.add_drops(3);
        let (clients, frames, drops) = t.snapshot();
        assert_eq!(clients, 1);
        assert_eq!(frames, 1);
        assert_eq!(drops, 3);
        t.dec_clients();
        assert_eq!(t.snapshot().0, 0);
    }
}
