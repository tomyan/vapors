//! A burst of wheel events scrolls one container, as Chromium latches
//! it: the container the burst first moves takes every event that comes
//! within `LATCH` of the one before, and an event past its end goes
//! nowhere; after a longer pause the next event finds its container
//! again, innermost first (F2c S9).

use std::time::Duration;

use vapors_dom::NodeId;
use vapors_elements::{Document, Point};
use vapors_platform::Instant;

/// How long a burst stays with its container after each event: what
/// Chromium 155 does on Linux. Measured 2026-10-25 under Xvfb with `XTest`
/// button-5 presses over a headful page holding a list inside a list:
/// with the inner one at its end, a notch 0.5 s after the last went
/// nowhere and one 0.55 s after scrolled the outer one.
pub(crate) const LATCH: Duration = Duration::from_millis(500);

/// The burst under way.
#[derive(Debug, Default)]
pub(crate) struct WheelScroll {
    latched: Option<(NodeId, Instant)>,
}

impl WheelScroll {
    /// A wheel event at `point` by `delta` (CSS pixels) at time `now`;
    /// whether a container moved.
    pub(crate) fn scroll(
        &mut self,
        document: &mut Document,
        point: Point,
        delta: [f32; 2],
        now: Instant,
    ) -> bool {
        let held = self
            .latched
            .filter(|(_, last)| now.duration_since(*last) <= LATCH)
            .map(|(container, _)| container);
        let Some(container) = held.or_else(|| document.scroll_target(point, delta)) else {
            self.latched = None;
            return false;
        };
        self.latched = Some((container, now));
        document.scroll_node_by(container, delta)
    }
}

#[cfg(test)]
#[path = "wheel_scroll_tests.rs"]
mod tests;
