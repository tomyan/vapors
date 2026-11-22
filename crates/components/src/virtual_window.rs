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

    /// How tall `rows` rows are, in px.
    ///
    /// # Panics
    /// When some count up to `rows` would round by more than
    /// `MAX_ROUNDING` (`holds`): layout would place rows off their
    /// pixel. Whether it panics depends on the count alone, rising: a
    /// count that holds, held at mount as the content's height, holds
    /// for every spacer height below it, so no scroll can panic.
    #[must_use]
    pub fn px(&self, rows: usize) -> f32 {
        assert!(
            rows <= MAX_ROWS,
            "VirtualList: {rows} rows is past 2^53, where a row count has no exact f64 and the height cannot be checked; use fewer rows (cargo test -p vapors-components virtual_window)"
        );
        assert!(
            self.holds(rows),
            "VirtualList: {rows} rows of {} px reach {} px, where an f32 places a row only to {} px (more than 1/128); use fewer rows, or a row height with a short binary fraction (33.25 or 33.5, not 33.3) (cargo test -p vapors-components virtual_window)",
            self.row_height,
            self.product(rows),
            self.half_step(rows)
        );
        narrow(self.product(rows))
    }

    /// Whether every count of rows up to `rows` has a height within
    /// `MAX_ROUNDING` of exact: each product is exact, which holds when
    /// the largest odd count's does (a product's odd part is its count's
    /// odd part times the row height's mantissa), or half the f32 step
    /// at `rows`' height, the most any product up to it rounds, is
    /// within the bound. Both only fail more as `rows` rises, as does the
    /// height overflowing an f32, which never holds.
    fn holds(&self, rows: usize) -> bool {
        let largest_odd = if rows.is_multiple_of(2) {
            rows.saturating_sub(1)
        } else {
            rows
        };
        // Only a count below 2^24 makes an exact product (its odd part
        // times a mantissa of at least 1 must fit an f32's 24 bits), and
        // below 2^24 the f64 product is the true one, so `exact` answers
        // truly; a larger count's product rounds in f64 too.
        let exact = largest_odd < 1 << 24 && self.exact(largest_odd);
        let finite = narrow(self.product(rows)).is_finite();
        finite && (exact || self.half_step(rows) <= MAX_ROUNDING)
    }

    /// The exact height of `rows` rows.
    fn product(&self, rows: usize) -> f64 {
        #[allow(
            clippy::cast_precision_loss,
            reason = "`px` refuses a count past MAX_ROWS, 2^53, before any product"
        )]
        let count = rows as f64;
        count * f64::from(self.row_height)
    }

    /// Whether `rows` rows' height is an f32 exactly.
    #[allow(clippy::float_cmp, reason = "exactness is the question")]
    fn exact(&self, rows: usize) -> bool {
        f64::from(narrow(self.product(rows))) == self.product(rows)
    }

    /// Half the f32 step at `rows` rows' height: the most a height there
    /// rounds.
    fn half_step(&self, rows: usize) -> f64 {
        let px = narrow(self.product(rows));
        f64::from(px.next_up() - px) / 2.0
    }
}

/// How far a height may round from the exact product of rows and row
/// height: 1/128 px, the resolution below which an f32 corner still
/// resolves a box (the elements crate frames a lane past 65,536 px for
/// the same reason). Paint snaps to device pixels, so an error this
/// small moves an edge by at most one device pixel, and only when the
/// exact edge lies within 1/128 px of a snapping boundary; a height of
/// 33.3 px rounds by about 1e-6 at three rows and is not refused.
const MAX_ROUNDING: f64 = 1.0 / 128.0;

/// The most rows `Window::px` checks: 2^53, the last count an f64 holds
/// exactly, so every product it forms is the exact one rounded once.
const MAX_ROWS: usize = 1 << 53;

/// `px` to the nearest f32.
fn narrow(px: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a height `Window::holds` bounds, narrowed once"
    )]
    let narrowed = px as f32;
    narrowed
}

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
