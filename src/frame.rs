use serde::{Deserialize, Serialize};

use crate::glyph::glyph_char;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameKind {
    Keyframe,
    Delta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellUpdate {
    pub x: u16,
    pub y: u16,
    pub glyph: u8,
    pub lum: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameEvent {
    pub frame_id: u64,
    pub speed_step: u8,
    pub width: u16,
    pub height: u16,
    pub kind: FrameKind,
    pub cells: Vec<CellUpdate>,
}

impl FrameEvent {
    pub fn stale_for(&self, last_frame_id: u64) -> bool {
        self.frame_id <= last_frame_id
    }

    pub fn apply_to(&self, buffer: &mut Vec<char>) {
        let needed = self.width as usize * self.height as usize;
        if buffer.len() != needed {
            *buffer = vec![' '; needed];
        }
        if self.kind == FrameKind::Keyframe {
            buffer.fill(' ');
        }
        for cell in &self.cells {
            if cell.x < self.width && cell.y < self.height {
                let idx = cell.y as usize * self.width as usize + cell.x as usize;
                if idx < buffer.len() {
                    buffer[idx] = glyph_char(cell.glyph);
                }
            }
        }
    }

    pub fn as_text(&self, buffer: &mut Vec<char>) -> String {
        self.apply_to(buffer);
        let mut out = String::with_capacity(buffer.len() + self.height as usize);
        for y in 0..self.height as usize {
            let start = y * self.width as usize;
            let end = start + self.width as usize;
            for ch in &buffer[start..end] {
                out.push(*ch);
            }
            if y + 1 != self.height as usize {
                out.push('\n');
            }
        }
        out
    }
}

pub fn resize_top_left<T: Copy + Default>(
    old: &[T],
    old_w: u16,
    old_h: u16,
    new_w: u16,
    new_h: u16,
) -> Vec<T> {
    let mut new_buf = vec![T::default(); new_w as usize * new_h as usize];
    let copy_h = old_h.min(new_h) as usize;
    let copy_w = old_w.min(new_w) as usize;

    for y in 0..copy_h {
        let old_start = y * old_w as usize;
        let new_start = y * new_w as usize;
        new_buf[new_start..new_start + copy_w].copy_from_slice(&old[old_start..old_start + copy_w]);
    }

    new_buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_stale_frames() {
        let frame = FrameEvent {
            frame_id: 10,
            speed_step: 16,
            width: 2,
            height: 1,
            kind: FrameKind::Delta,
            cells: vec![],
        };
        assert!(frame.stale_for(10));
        assert!(frame.stale_for(11));
        assert!(!frame.stale_for(9));
    }

    #[test]
    fn resize_clips_and_extends_from_top_left() {
        let old = vec!['a', 'b', 'c', 'd'];
        let clipped = resize_top_left(&old, 2, 2, 1, 1);
        assert_eq!(clipped, vec!['a']);

        let extended = resize_top_left(&old, 2, 2, 3, 3);
        assert_eq!(extended[0], 'a');
        assert_eq!(extended[1], 'b');
        assert_eq!(extended[3], 'c');
        assert_eq!(extended[4], 'd');
    }
}
