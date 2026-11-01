//! How many presses in a row a press of the primary button is (E6a),
//! counted natively: one soon after and near the last counts one more,
//! so a second is a double click. The web page counts its own presses
//! (`mousedown`'s `detail`), so this is native only.
//!
//! Each press is stamped when the platform receives it from winit, whose
//! events carry no timestamp of their own; that is before the app
//! handles it, so a slow frame does not run two presses together.
//!
//! The interval and distance are fallbacks, not the system's settings,
//! which no build here compiles a reader for (Windows'
//! `GetDoubleClickTime` and `SM_CXDOUBLECLK`, macOS's `NSEvent`
//! `doubleClickInterval`, the desktop's own on Linux); a CI target that
//! compiles those readers retires them.

use std::num::NonZeroU32;
use std::time::Duration;

use crate::clock::Instant;

/// Fallback: GTK's defaults (`gtk-double-click-time` 400 ms,
/// `gtk-double-click-distance` 5 px), which Linux Chromium reads.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const FALLBACK: (Duration, f32) = (Duration::from_millis(400), 5.0);

/// Fallback: Windows' defaults (`GetDoubleClickTime` 500 ms,
/// `SM_CXDOUBLECLK` 4 px).
#[cfg(target_os = "windows")]
const FALLBACK: (Duration, f32) = (Duration::from_millis(500), 4.0);

/// Fallback: macOS's default `doubleClickInterval` (0.5 s); macOS has no
/// distance setting, so Windows' 4 px.
#[cfg(target_os = "macos")]
const FALLBACK: (Duration, f32) = (Duration::from_millis(500), 4.0);

/// The last press, and its count.
#[derive(Default)]
pub(crate) struct Clicks {
    last: Option<([f32; 2], Instant, NonZeroU32)>,
}

impl Clicks {
    /// A press at `at` (CSS px) received at `now`: its count in the row.
    pub(crate) fn pressed(&mut self, at: [f32; 2], now: Instant) -> NonZeroU32 {
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
fn follows(last_at: [f32; 2], last_now: Instant, at: [f32; 2], now: Instant) -> bool {
    let (interval, distance) = FALLBACK;
    let near = (at[0] - last_at[0]).abs() <= distance && (at[1] - last_at[1]).abs() <= distance;
    near && now.saturating_duration_since(last_now) <= interval
}

#[cfg(test)]
#[path = "clicks_tests.rs"]
mod tests;
