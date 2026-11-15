//! Flinging a scroll container: started at a velocity, or by a touch
//! gesture's finger lifting while it moves (`Event::Fling`), moved on at
//! each frame (`DocumentInput::advance_fling`), and stopped by the next
//! finger down, a wheel scroll or a button press (`touch_scroll.rs`,
//! `document_input_act.rs`). Losing the window's focus does not stop it,
//! as Chromium does not stop a fling on blur.

use std::fmt;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::{Instant, report_error};

use super::DocumentInput;
use crate::fling::Fling;
use crate::fling_curve::FlingCurve;

/// Why a fling could not start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlingError {
    /// A velocity component is NaN or infinite.
    UnreadableVelocity([f32; 2]),
    /// The element is not a scroll container.
    NotAScroller(NodeId),
    /// A finger is down: Chromium starts a fling only from a gesture's
    /// end, and a finger down stops one.
    FingerDown,
}

impl fmt::Display for FlingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnreadableVelocity(velocity) => {
                write!(f, "a fling's velocity must be finite, not {velocity:?}")
            }
            Self::NotAScroller(node) => write!(
                f,
                "{node:?} is not a scroll container (give it `overflow: auto` or `scroll`), so it cannot be flung"
            ),
            Self::FingerDown => write!(
                f,
                "a fling cannot start while a finger is down; start it when the last finger lifts"
            ),
        }
    }
}

impl std::error::Error for FlingError {}

/// Each axis of a fling's velocity is clamped to this, CSS px/s, as
/// Chromium's `GestureDetector` clamps the release velocity to the
/// maximum fling velocity (`ComputeCurrentVelocity(1000, max)`,
/// `ui/events/gesture_detection/gesture_detector.cc:427`, from
/// Android's `ViewConfiguration`). AOSP's maximum is 8000 dp/s, but the
/// Pixel flung at 13 214 and 15 502 (010, 016; docs/plans/
/// touch-momentum.md), so 8000 would cut those flings short: this is
/// the measured lower bound, rounded up. Provisional until #400 reads
/// the Pixel's real maximum.
pub const MAX_FLING_SPEED: f32 = 16_000.0;
/// Below this speed on both axes a lifted finger does not fling, CSS
/// px/s: Chromium's `GestureDetector` takes Android's
/// `ViewConfiguration` minimum, 50 dp/s in AOSP.
const MIN_FLING_SPEED: f32 = 50.0;

impl DocumentInput {
    /// The gesture's finger lifted moving at `velocity` (CSS px/s, the
    /// scroll's way): the container the gesture scrolled flings, as
    /// Chromium's `GestureDetector` decides on `Action::UP`. The thumbs,
    /// hidden at the lift, show again for the fling: that change asks
    /// the host for the frame that moves it on.
    ///
    /// RELAXED: the fling starts when the event is handled, where
    /// Chromium starts it at the fling event's time; winit gives touches
    /// no timestamp (as `Input::counted` says for clicks). Retired when
    /// winit exposes event times.
    pub(super) fn released(&mut self, document: &Document, velocity: [f32; 2]) {
        let velocity = velocity.map(|v| v.clamp(-MAX_FLING_SPEED, MAX_FLING_SPEED));
        if velocity.iter().all(|v| v.abs() <= MIN_FLING_SPEED) {
            return;
        }
        let Some(container) = self.touch.released() else {
            return;
        };
        if let Err(refused) = self.fling(document, container, velocity, Instant::now()) {
            report_error(&format!("a touch fling did not start: {refused}"));
        }
    }

    /// `container` flung at `velocity` (CSS px/s, the way its scroll
    /// offset moves; each axis clamped to [`MAX_FLING_SPEED`]) from
    /// `at`, replacing a fling under way; a zero velocity starts none.
    ///
    /// # Errors
    ///
    /// [`FlingError`] when the velocity is not finite, `container` is
    /// not a scroll container, or a finger is down; nothing changes.
    pub fn fling(
        &mut self,
        document: &Document,
        container: NodeId,
        velocity: [f32; 2],
        at: Instant,
    ) -> Result<(), FlingError> {
        if !velocity.iter().all(|v| v.is_finite()) {
            return Err(FlingError::UnreadableVelocity(velocity));
        }
        if document.scroll_offset(container).is_none() {
            return Err(FlingError::NotAScroller(container));
        }
        if self.touch.finger_down() {
            return Err(FlingError::FingerDown);
        }
        let velocity = velocity.map(|v| v.clamp(-MAX_FLING_SPEED, MAX_FLING_SPEED));
        if let Some(curve) = FlingCurve::new(velocity) {
            self.touch.fling(Fling::new(container, curve, at));
        }
        Ok(())
    }

    /// The fling under way moved on to `now`; whether one is still
    /// running, so the host draws another frame. An app showing the
    /// document calls this once a frame, before the document's update.
    pub fn advance_fling(&mut self, document: &mut Document, now: Instant) -> bool {
        let running = self.touch.advance(document, now);
        document.set_thumbs_shown(self.touch.thumbs_shown());
        running
    }

    /// Whether a fling is running.
    #[must_use]
    pub fn flinging(&self) -> bool {
        self.touch.flinging()
    }
}

#[cfg(test)]
#[path = "document_input_fling_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "document_input_release_tests.rs"]
mod release_tests;
