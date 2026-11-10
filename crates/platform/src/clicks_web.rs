//! The click fallback on the web: the browser's platform
//! (`browser_platform`) picks Windows', macOS's or else GTK's (Linux,
//! Chrome OS, Android) stated fallback (`clicks`).

use std::time::Duration;

/// The fallback for the browser's platform.
pub(crate) fn fallback() -> (Duration, f32) {
    crate::clicks::by_platform(&crate::browser_platform::platform())
}
