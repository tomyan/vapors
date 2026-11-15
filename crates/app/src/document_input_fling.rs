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

/// Each axis of a lifted finger's velocity is clamped to this, CSS
/// px/s: Android's `ViewConfiguration` maximum fling velocity, 8000 dp/s
/// in AOSP, which Chromium's `GestureDetector` clamps the release
/// velocity to (`ComputeCurrentVelocity(1000, max)`,
/// `ui/events/gesture_detection/gesture_detector.cc:427`). A boost adds
/// the previous fling's start after this clamp and is not clamped
/// (`FlingBooster::GetVelocityForFlingStart`,
/// `ui/events/blink/fling_booster.cc`, called from
/// `FlingController::UpdateCurrentFlingState`). The Pixel's fastest
/// starts, 13 264 and 15 562 (010, 016), were boosts: 4436 and 4608 on
/// 8828 and 10 954. #400 confirms 8000 on the device.
pub const MAX_FLING_SPEED: f32 = 8000.0;
/// Below this speed on both axes a lifted finger does not fling, CSS
/// px/s: Chromium's `GestureDetector` takes Android's
/// `ViewConfiguration` minimum, 50 dp/s in AOSP.
const MIN_FLING_SPEED: f32 = 50.0;

/// `velocity` with each axis clamped to [`MAX_FLING_SPEED`].
fn clamped(velocity: [f32; 2]) -> [f32; 2] {
    velocity.map(|v| v.clamp(-MAX_FLING_SPEED, MAX_FLING_SPEED))
}

impl DocumentInput {
    /// The gesture's finger lifted at `at` moving at `velocity` (CSS
    /// px/s, the scroll's way): the container the gesture scrolled
    /// flings from then, as Chromium's `GestureDetector` decides on
    /// `Action::UP`. Starting it asks the host for the frame that moves
    /// it on (`DocumentInput::handle`). Its time is the platform's handling time (`velocity_tracker.rs`
    /// says why), where Chromium starts at the last scroll update's.
    pub(super) fn released(&mut self, document: &Document, velocity: [f32; 2], at: Instant) {
        let velocity = clamped(velocity);
        if velocity.iter().all(|v| v.abs() <= MIN_FLING_SPEED) {
            return;
        }
        let Some(container) = self.touch.take_released() else {
            return;
        };
        // a boost adds the last start after the clamp, unclamped, as
        // `FlingBooster` adds it after `GestureDetector` clamps
        let boosted = self.touch.boosted(velocity, at);
        if let Err(refused) = self.start(document, container, boosted, at) {
            report_error(&format!("a touch fling did not start: {refused}"));
        }
    }

    /// `container` flung at `velocity` (CSS px/s, the way its scroll
    /// offset moves; each axis clamped to [`MAX_FLING_SPEED`]) from
    /// `at`, replacing a fling under way; a zero velocity starts none. It
    /// is not a touch's fling, so it boosts nothing and is not boosted.
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
        if self.touch.finger_down() {
            return Err(FlingError::FingerDown);
        }
        self.start(document, container, clamped(velocity), at)?;
        self.touch.forget_boost();
        Ok(())
    }

    /// The fling of `container` at `velocity` from `at` starts.
    fn start(
        &mut self,
        document: &Document,
        container: NodeId,
        velocity: [f32; 2],
        at: Instant,
    ) -> Result<(), FlingError> {
        if document.scroll_offset(container).is_none() {
            return Err(FlingError::NotAScroller(container));
        }
        if let Some(curve) = FlingCurve::new(velocity) {
            self.touch.fling(Fling::new(container, curve, at));
        }
        Ok(())
    }

    /// The fling under way moved on to `now`, and the touch thumbs'
    /// fade with it; whether a fling is still running, so the host draws
    /// another frame. An app showing the document calls this once a
    /// frame, before the document's update.
    pub fn advance_fling(&mut self, document: &mut Document, now: Instant) -> bool {
        let running = self.touch.advance(document, now);
        self.touch.draw_thumbs(document, now);
        self.thumbs = self.touch.thumb_phase(now);
        running
    }

    /// Where the touch thumbs' fade was at the last `advance_fling`: the
    /// host draws a frame at once while they fade, and at the fade's
    /// start while shown.
    #[must_use]
    pub(crate) fn thumb_phase(&self) -> crate::thumb_fade::Phase {
        self.thumbs
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
