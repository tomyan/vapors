//! A clip as the GPU's scissor rectangle: whole pixels inside the
//! target.

use crate::{Rect, Size};

/// A scissor rectangle in whole device pixels, inside the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scissor {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width, at least 1.
    pub width: u32,
    /// Height, at least 1.
    pub height: u32,
}

impl Rect {
    /// This clip as a scissor on a `viewport`-sized target: each edge
    /// rounded to the nearest pixel and cut to the target; `None` when
    /// nothing of it is left, so nothing under it draws.
    #[must_use]
    pub fn scissor(&self, viewport: Size) -> Option<Scissor> {
        let (left, right) = span(self.left, self.right, viewport.width)?;
        let (top, bottom) = span(self.top, self.bottom, viewport.height)?;
        Some(Scissor {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
}

impl Scissor {
    /// Where both scissors let drawing through; `None` when nowhere.
    #[must_use]
    pub fn intersect(self, other: Scissor) -> Option<Scissor> {
        let (x, right) = overlap(self.x, self.width, other.x, other.width)?;
        let (y, bottom) = overlap(self.y, self.height, other.y, other.height)?;
        Some(Scissor {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

/// The overlap of `a..a + a_len` and `b..b + b_len`; `None` when empty.
fn overlap(a: u32, a_len: u32, b: u32, b_len: u32) -> Option<(u32, u32)> {
    let (start, end) = (a.max(b), (a + a_len).min(b + b_len));
    (start < end).then_some((start, end))
}

/// `start..end` rounded and cut to `0..limit`; `None` when empty.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // clamped to 0..=limit first; limits are far below 2^24
fn span(start: f32, end: f32, limit: u32) -> Option<(u32, u32)> {
    let cut = |edge: f32| edge.round().clamp(0.0, limit as f32) as u32;
    let (start, end) = (cut(start), cut(end));
    (start < end).then_some((start, end))
}

#[cfg(test)]
#[path = "scissor_tests.rs"]
mod tests;
