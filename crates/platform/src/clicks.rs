//! How many presses in a row a press of the primary button is (E6a),
//! counted by the platform on every target, the web included (winit
//! cancels `pointerdown`, so the browser's `mousedown` count never
//! comes): one soon after and near the last counts one more, so a
//! second is a double click.
//!
//! The interval and distance are fallbacks, not the system's settings,
//! which no build here compiles a reader for (Windows'
//! `GetDoubleClickTime` and `SM_CXDOUBLECLK`, macOS's `NSEvent`
//! `doubleClickInterval`, the desktop's own on Linux); a CI target that
//! compiles those readers retires them. Natively the fallback is the
//! target OS's; on the web, the browser's platform's (`clicks_web`).

use std::num::NonZeroU32;
use std::time::Duration;

use crate::clock::Instant;

/// Fallback: GTK's defaults (`gtk-double-click-time` 400 ms,
/// `gtk-double-click-distance` 5 px), which Linux Chromium reads.
pub(crate) const GTK: (Duration, f32) = (Duration::from_millis(400), 5.0);

/// Fallback: Windows' defaults (`GetDoubleClickTime` 500 ms,
/// `SM_CXDOUBLECLK` 4 px).
pub(crate) const WINDOWS: (Duration, f32) = (Duration::from_millis(500), 4.0);

/// Fallback: macOS's default `doubleClickInterval` (0.5 s); macOS has no
/// distance setting, so Windows' 4 px.
pub(crate) const MACOS: (Duration, f32) = (Duration::from_millis(500), 4.0);

/// The fallback for this target: its OS's, or on the web its browser's
/// platform's.
pub(crate) fn fallback() -> (Duration, f32) {
    #[cfg(target_family = "wasm")]
    return crate::clicks_web::fallback();
    #[cfg(target_os = "windows")]
    return WINDOWS;
    #[cfg(target_os = "macos")]
    return MACOS;
    #[cfg(not(any(target_family = "wasm", target_os = "windows", target_os = "macos")))]
    GTK
}

/// The fallback a browser's platform names (`navigator.userAgentData`
/// `.platform`, else its user agent): Windows', macOS's (iPhone and
/// iPad too), else GTK's.
// natively its tests run it; only the web reads a browser's platform
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
pub(crate) fn by_platform(platform: &str) -> (Duration, f32) {
    let platform = platform.to_ascii_lowercase();
    if platform.contains("win") {
        WINDOWS
    } else if ["mac", "iphone", "ipad"]
        .iter()
        .any(|name| platform.contains(name))
    {
        MACOS
    } else {
        GTK
    }
}

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
    let (interval, distance) = fallback();
    let near = (at[0] - last_at[0]).abs() <= distance && (at[1] - last_at[1]).abs() <= distance;
    near && now.saturating_duration_since(last_now) <= interval
}

#[cfg(test)]
#[path = "clicks_tests.rs"]
mod tests;
