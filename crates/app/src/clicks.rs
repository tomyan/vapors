//! How many presses in a row a press is: one soon after and near the
//! last counts one more, so a second is a double click (E6a). The
//! numbers are Windows' defaults (`GetDoubleClickTime` 500 ms,
//! `SM_CXDOUBLECLK` and `SM_CYDOUBLECLK` 4 px); vapors does not read
//! the system's own setting yet.

use std::num::NonZeroU32;
use std::time::Duration;

use vapors_platform::Instant;

use vapors_elements::Point;

/// A press later than this after the last starts a new row.
const DOUBLE_CLICK_TIME: Duration = Duration::from_millis(500);
/// A press further than this across or down (CSS px) from the last
/// starts a new row.
const DOUBLE_CLICK_SLOP: f32 = 4.0;

/// The last press, and its count.
#[derive(Default)]
pub(crate) struct Clicks {
    last: Option<(Point, Instant, NonZeroU32)>,
}

impl Clicks {
    /// A press at `at` at `now`: its count in the row.
    pub(crate) fn pressed(&mut self, at: Point, now: Instant) -> NonZeroU32 {
        let count = match self.last {
            Some((last_at, last_now, count)) if follows(last_at, last_now, at, now) => {
                count.saturating_add(1)
            }
            _ => NonZeroU32::MIN,
        };
        self.last = Some((at, now, count));
        count
    }
}

/// Whether a press at `at`, `now` continues a row last pressed at
/// `last_at`, `last_now`.
fn follows(last_at: Point, last_now: Instant, at: Point, now: Instant) -> bool {
    let near = (at[0] - last_at[0]).abs() <= DOUBLE_CLICK_SLOP
        && (at[1] - last_at[1]).abs() <= DOUBLE_CLICK_SLOP;
    near && now.saturating_duration_since(last_now) <= DOUBLE_CLICK_TIME
}

#[cfg(test)]
#[path = "clicks_tests.rs"]
mod tests;
