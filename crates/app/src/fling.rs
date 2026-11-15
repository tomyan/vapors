//! A fling under way: one scroll container moved along a
//! [`FlingCurve`] at each frame, as Chromium's `FlingController` moves
//! it (`components/input/fling_controller.cc`, `ProgressFling`): nothing
//! until a frame after the fling's start, then the curve's offset since
//! the last frame as a scroll delta, each delta of 0.1 px or less on
//! both axes left unsent (`kMinInertialScrollDelta`), and no delta at all
//! once the curve has ended. A delta that moves nothing (the content's
//! edge) ends the fling, as an unconsumed momentum scroll update does
//! (`StopFlingingIfNecessary`, `components/input/
//! render_widget_host_view_input.cc`).

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::Instant;

use crate::fling_curve::FlingCurve;

/// `kMinInertialScrollDelta`: a delta this small on both axes is not
/// sent, CSS px.
const MIN_DELTA: f32 = 0.1;

/// One fling.
#[derive(Debug)]
pub(crate) struct Fling {
    container: NodeId,
    start: Instant,
    curve: FlingCurve,
    /// The curve's offset at the last frame, sent or not.
    reached: [f32; 2],
}

impl Fling {
    /// A fling of `container` along `curve` from `start`.
    pub(crate) fn new(container: NodeId, curve: FlingCurve, start: Instant) -> Self {
        Self {
            container,
            start,
            curve,
            reached: [0.0; 2],
        }
    }

    /// The fling moved on to `now`; whether it is still running.
    pub(crate) fn advance(&mut self, document: &mut Document, now: Instant) -> bool {
        let Some(elapsed) = now
            .checked_duration_since(self.start)
            .filter(|e| !e.is_zero())
        else {
            return true;
        };
        let Some(offset) = self.curve.offset_at(elapsed) else {
            return false;
        };
        let delta = [offset[0] - self.reached[0], offset[1] - self.reached[1]];
        self.reached = offset;
        if delta.iter().all(|d| d.abs() <= MIN_DELTA) {
            return true;
        }
        document.scroll_node_by(self.container, delta)
    }
}

#[cfg(test)]
#[path = "fling_tests.rs"]
mod tests;
