//! Which rows of a fixed-row-height virtual list exist at a scroll
//! offset, and where they sit (`virtual_list.wisp`). The numbers that
//! would place rows wrongly are refused, naming the prop.

use std::ops::Range;

/// The geometry of a virtual list: every row `row_height` px tall, a
/// window `height` px tall, and `overscan` rows kept on each side of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    row_height: f32,
    height: f32,
    overscan: usize,
}

impl Window {
    /// The geometry; refuses a row height that is not a positive finite
    /// number and a window height that is negative or not finite.
    ///
    /// # Panics
    /// On those numbers, naming the prop.
    #[must_use]
    pub fn new(row_height: f32, height: f32, overscan: usize) -> Window {
        assert!(
            row_height.is_finite() && row_height > 0.0,
            "VirtualList: row_height is {row_height}; give every row's height in px, above 0 (cargo test -p vapors-components virtual_window)"
        );
        assert!(
            height.is_finite() && height >= 0.0,
            "VirtualList: height is {height}; give the list's height in px, 0 or more (cargo test -p vapors-components virtual_window)"
        );
        Window {
            row_height,
            height,
            overscan,
        }
    }

    /// The rows of `count` that exist at vertical scroll offset `offset`:
    /// those any part of which is in the window, and `overscan` more on
    /// each side, cut to `0..count`.
    ///
    /// # Panics
    /// On an offset that is negative or not finite: a scroll container's
    /// offset never is.
    #[must_use]
    pub fn rows(&self, offset: f32, count: usize) -> Range<usize> {
        assert!(
            offset.is_finite() && offset >= 0.0,
            "VirtualList: scroll offset {offset} is not a clamped offset (cargo test -p vapors-components virtual_window)"
        );
        let first = whole_rows(offset / self.row_height, f32::floor);
        let past = whole_rows((offset + self.height) / self.row_height, f32::ceil);
        let end = past.saturating_add(self.overscan).min(count);
        first.saturating_sub(self.overscan).min(end)..end
    }

    /// How tall `rows` rows are, in px, within `MAX_ROUNDING` of the
    /// exact product.
    ///
    /// # Panics
    /// When the nearest `f32` is further off: layout would place the
    /// rows below it off their pixel.
    #[must_use]
    pub fn px(&self, rows: usize) -> f32 {
        #[allow(
            clippy::cast_precision_loss,
            reason = "checked below: the f32 must lie within MAX_ROUNDING of the exact product"
        )]
        let exact = rows as f64 * f64::from(self.row_height);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "checked below: the f32 must lie within MAX_ROUNDING of the exact product"
        )]
        let px = exact as f32;
        let off = (f64::from(px) - exact).abs();
        assert!(
            off <= MAX_ROUNDING,
            "VirtualList: {rows} rows of {} px is {exact} px, which an f32 layout holds only to {off} px (more than 1/128); use fewer rows or a row height with fewer significant bits (cargo test -p vapors-components virtual_window)",
            self.row_height
        );
        px
    }
}

/// How far a height may round from the exact product of rows and row
/// height: 1/128 px, the resolution below which an f32 corner still
/// resolves a box (the elements crate frames a lane past 65,536 px for
/// the same reason). Paint snaps to device pixels, so an error this
/// small moves nothing; a height of 33.3 px rounds by about 1e-6 at
/// three rows, and must not be refused.
const MAX_ROUNDING: f64 = 1.0 / 128.0;

/// `rows` rounded by `round` to a row index, saturating at `usize::MAX`.
fn whole_rows(rows: f32, round: fn(f32) -> f32) -> usize {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a finite, non-negative count of rows; `as` saturates past usize::MAX"
    )]
    let whole = round(rows) as usize;
    whole
}

#[cfg(test)]
#[path = "virtual_window_tests.rs"]
mod tests;
