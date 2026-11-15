//! A fling under way: one scroll container moved along a
//! [`FlingCurve`] at each frame, as Chromium's `FlingController` moves
//! it (`components/input/fling_controller.cc:197-270`, `ProgressFling`,
//! read at chromium 1555c8bf):
//!
//! - until a delta has been sent, a frame two frames' time or more after
//!   the start moves the start to one frame before it, so a fling first
//!   drawn late does not jump (`kMaxMicrosecondsFromFlingTimestampToFirstProgress`);
//! - a frame at or before the start, or before the last frame that sent
//!   a delta, moves nothing;
//! - otherwise the curve's offset since the last frame is the delta,
//!   unsent when 0.1 px or less on both axes (`kMinInertialScrollDelta`);
//! - once the curve has ended no delta is sent and the fling ends;
//! - a delta that moves nothing (the content's edge) ends the fling, as
//!   an unconsumed momentum scroll update does
//!   (`StopFlingingIfNecessary`, `components/input/
//!   render_widget_host_view_input.cc:264-282`).

use std::time::Duration;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::Instant;

use crate::fling_curve::FlingCurve;

/// `kMinInertialScrollDelta`: a delta this small on both axes is not
/// sent, CSS px.
const MIN_DELTA: f32 = 0.1;
/// `kFrameDelta`: one frame at 60 Hz.
const FRAME: Duration = Duration::from_micros(16_667);
/// `kMaxMicrosecondsFromFlingTimestampToFirstProgress`: two frames.
const LATE_FIRST_FRAME: Duration = Duration::from_micros(33_333);

/// One fling.
#[derive(Debug)]
pub(crate) struct Fling {
    container: NodeId,
    start: Instant,
    curve: FlingCurve,
    /// The curve's offset at the last frame, sent or not.
    reached: [f32; 2],
    /// The last frame that sent a delta (`last_progress_time_`).
    last_sent: Option<Instant>,
}

impl Fling {
    /// A fling of `container` along `curve` from `start`.
    pub(crate) fn new(container: NodeId, curve: FlingCurve, start: Instant) -> Self {
        Self {
            container,
            start,
            curve,
            reached: [0.0; 2],
            last_sent: None,
        }
    }

    /// The fling moved on to `now`; whether it is still running.
    pub(crate) fn advance(&mut self, document: &mut Document, now: Instant) -> bool {
        if self.last_sent.is_none() && now >= self.start + LATE_FIRST_FRAME {
            // now is two frames past the start, so one frame back is after it
            self.start = now.checked_sub(FRAME).unwrap_or(self.start);
        }
        let behind = self.last_sent.is_some_and(|sent| now < sent);
        let Some(elapsed) = now
            .checked_duration_since(self.start)
            .filter(|e| !e.is_zero() && !behind)
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
        self.last_sent = Some(now);
        document.scroll_node_by(self.container, delta)
    }
}

#[cfg(test)]
#[path = "fling_tests.rs"]
mod tests;
