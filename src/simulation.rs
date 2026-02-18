use rand::Rng;

use crate::config::DEFAULT_FPS;
use crate::frame::{CellUpdate, FrameEvent, FrameKind, resize_top_left};
use crate::glyph::SPACE_GLYPH;

pub const NUMERIC_WEIGHT: u32 = 60;
pub const ALPHA_WEIGHT: u32 = 20;
pub const KATAKANA_WEIGHT: u32 = 20;
const KEYFRAME_INTERVAL: u64 = 30;
const GLYPH_HOLD_PROBABILITY: f64 = 0.90;

const NUMERIC_START: u8 = 1;
const NUMERIC_LEN: u8 = 10;
const ALPHA_START: u8 = NUMERIC_START + NUMERIC_LEN;
const ALPHA_LEN: u8 = 26;
const KATAKANA_START: u8 = ALPHA_START + ALPHA_LEN;
const KATAKANA_LEN: u8 = 46;

#[derive(Debug, Clone)]
struct Column {
    head: i32,
    len: u16,
    stride: u8,
}

#[derive(Debug)]
pub struct Simulation {
    width: u16,
    height: u16,
    fps: f32,
    frame_id: u64,
    columns: Vec<Column>,
    grid_glyph: Vec<u8>,
    prev_glyph: Vec<u8>,
    grid_lum: Vec<u8>,
    prev_lum: Vec<u8>,
}

impl Simulation {
    pub fn new(width: u16, height: u16, fps: f32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let mut sim = Self {
            width,
            height,
            fps: if fps <= 0.0 { DEFAULT_FPS } else { fps },
            frame_id: 0,
            columns: Vec::new(),
            grid_glyph: vec![SPACE_GLYPH; width as usize * height as usize],
            prev_glyph: vec![SPACE_GLYPH; width as usize * height as usize],
            grid_lum: vec![0; width as usize * height as usize],
            prev_lum: vec![0; width as usize * height as usize],
        };
        sim.reseed_columns();
        sim
    }

    pub fn target_fps(&self) -> f32 {
        self.fps
    }

    pub fn keyframe(&self) -> FrameEvent {
        FrameEvent {
            frame_id: self.frame_id,
            speed_step: 16,
            width: self.width,
            height: self.height,
            kind: FrameKind::Keyframe,
            cells: self
                .grid_glyph
                .iter()
                .enumerate()
                .map(|(idx, glyph)| CellUpdate {
                    x: (idx % self.width as usize) as u16,
                    y: (idx / self.width as usize) as u16,
                    glyph: *glyph,
                    lum: self.grid_lum[idx],
                })
                .collect(),
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        let width = width.max(1);
        let height = height.max(1);
        if width == self.width && height == self.height {
            return;
        }

        self.grid_glyph = resize_top_left(&self.grid_glyph, self.width, self.height, width, height);
        self.prev_glyph = resize_top_left(&self.prev_glyph, self.width, self.height, width, height);
        self.grid_lum = resize_top_left(&self.grid_lum, self.width, self.height, width, height);
        self.prev_lum = resize_top_left(&self.prev_lum, self.width, self.height, width, height);

        if width > self.width {
            let mut rng = rand::rng();
            for _ in self.width..width {
                self.columns.push(Self::random_column(height, &mut rng));
            }
        } else {
            self.columns.truncate(width as usize);
        }

        self.width = width;
        self.height = height;
    }

    pub fn grow_to_fit(&mut self, width: u16, height: u16) {
        let target_w = self.width.max(width.max(1));
        let target_h = self.height.max(height.max(1));
        if target_w == self.width && target_h == self.height {
            return;
        }
        self.resize(target_w, target_h);
    }

    pub fn tick(&mut self) -> FrameEvent {
        self.frame_id += 1;
        self.prev_glyph.copy_from_slice(&self.grid_glyph);
        self.prev_lum.copy_from_slice(&self.grid_lum);
        self.grid_glyph.fill(SPACE_GLYPH);
        self.grid_lum.fill(0);

        let mut rng = rand::rng();
        for (x, col) in self.columns.iter_mut().enumerate() {
            if self.frame_id % col.stride as u64 == 0 {
                col.head += 1;
            }
            if col.head - col.len as i32 > self.height as i32 {
                *col = Self::random_column(self.height, &mut rng);
            }

            let start = (col.head - col.len as i32 + 1).max(0);
            let end = col.head.min(self.height as i32 - 1);
            let run_len = (end - start + 1).max(1) as f32;
            for y in start..=end {
                let idx = y as usize * self.width as usize + x;
                if idx < self.grid_glyph.len() {
                    let from_head = (end - y) as f32;
                    self.grid_glyph[idx] = if self.prev_glyph[idx] != SPACE_GLYPH
                        && rng.random_bool(GLYPH_HOLD_PROBABILITY)
                    {
                        self.prev_glyph[idx]
                    } else {
                        sample_glyph_index(&mut rng)
                    };
                    self.grid_lum[idx] = luminance_for_trail(run_len, from_head);
                }
            }
        }

        let kind = if self.frame_id % KEYFRAME_INTERVAL == 0 {
            FrameKind::Keyframe
        } else {
            FrameKind::Delta
        };

        let mut cells = Vec::new();
        match kind {
            FrameKind::Keyframe => {
                for (idx, glyph) in self.grid_glyph.iter().enumerate() {
                    cells.push(CellUpdate {
                        x: (idx % self.width as usize) as u16,
                        y: (idx / self.width as usize) as u16,
                        glyph: *glyph,
                        lum: self.grid_lum[idx],
                    });
                }
            }
            FrameKind::Delta => {
                for idx in 0..self.grid_glyph.len() {
                    if self.grid_glyph[idx] != self.prev_glyph[idx] || self.grid_lum[idx] != self.prev_lum[idx] {
                        cells.push(CellUpdate {
                            x: (idx % self.width as usize) as u16,
                            y: (idx / self.width as usize) as u16,
                            glyph: self.grid_glyph[idx],
                            lum: self.grid_lum[idx],
                        });
                    }
                }
            }
        }

        FrameEvent {
            frame_id: self.frame_id,
            speed_step: 16,
            width: self.width,
            height: self.height,
            kind,
            cells,
        }
    }

    fn reseed_columns(&mut self) {
        let mut rng = rand::rng();
        self.columns = (0..self.width)
            .map(|_| Self::random_column(self.height, &mut rng))
            .collect();
    }

    fn random_column(height: u16, rng: &mut impl Rng) -> Column {
        let len = rng.random_range(4..=height.max(4).min(32));
        let stride = rng.random_range(1..=4);
        let head = -rng.random_range(0..height as i32);
        Column { head, len, stride }
    }
}

fn sample_glyph_index(rng: &mut impl Rng) -> u8 {
    let total = NUMERIC_WEIGHT + ALPHA_WEIGHT + KATAKANA_WEIGHT;
    let pick = rng.random_range(0..total);
    if pick < NUMERIC_WEIGHT {
        NUMERIC_START + rng.random_range(0..NUMERIC_LEN)
    } else if pick < NUMERIC_WEIGHT + ALPHA_WEIGHT {
        ALPHA_START + rng.random_range(0..ALPHA_LEN)
    } else {
        KATAKANA_START + rng.random_range(0..KATAKANA_LEN)
    }
}

fn luminance_for_trail(run_len: f32, from_head: f32) -> u8 {
    if run_len <= 1.0 || from_head == 0.0 {
        return 255;
    }
    let t = (from_head / run_len).clamp(0.0, 1.0);
    if t <= 0.8 {
        let p = t / 0.8;
        (190.0 - (70.0 * p)).round() as u8
    } else {
        let p = (t - 0.8) / 0.2;
        (120.0 - (105.0 * p)).round().max(8.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_expected_weights() {
        assert_eq!(NUMERIC_WEIGHT, 60);
        assert_eq!(ALPHA_WEIGHT, 20);
        assert_eq!(KATAKANA_WEIGHT, 20);
    }

    #[test]
    fn resize_preserves_top_left_data() {
        let mut sim = Simulation::new(2, 2, 60.0);
        sim.grid_glyph = vec![1, 2, 3, 4];
        sim.prev_glyph = sim.grid_glyph.clone();
        sim.resize(1, 1);
        assert_eq!(sim.grid_glyph, vec![1]);
        sim.resize(3, 3);
        assert_eq!(sim.grid_glyph[0], 1);
    }

    #[test]
    fn frame_id_is_monotonic() {
        let mut sim = Simulation::new(10, 10, 60.0);
        let a = sim.tick();
        let b = sim.tick();
        assert!(b.frame_id > a.frame_id);
    }

    #[test]
    fn emits_sparse_deltas_and_periodic_keyframes() {
        let mut sim = Simulation::new(4, 4, 60.0);
        let first = sim.tick();
        assert_eq!(first.kind, FrameKind::Delta);
        assert!(first.cells.len() <= 16);

        let mut thirtieth = first;
        for _ in 1..30 {
            thirtieth = sim.tick();
        }
        assert_eq!(thirtieth.frame_id, 30);
        assert_eq!(thirtieth.kind, FrameKind::Keyframe);
    }

    #[test]
    fn grow_to_fit_never_shrinks_dimensions() {
        let mut sim = Simulation::new(20, 10, 60.0);
        sim.grow_to_fit(5, 5);
        assert_eq!(sim.width, 20);
        assert_eq!(sim.height, 10);
        sim.grow_to_fit(40, 15);
        assert_eq!(sim.width, 40);
        assert_eq!(sim.height, 15);
    }
}
