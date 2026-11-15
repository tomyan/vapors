//! Flinging a scroll container: started at a velocity, moved on at each
//! frame (`DocumentInput::advance_fling`), and stopped by the next
//! finger down (`touch_scroll.rs`).

use std::fmt;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::Instant;

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
        }
    }
}

impl std::error::Error for FlingError {}

impl DocumentInput {
    /// `container` flung at `velocity` (CSS px/s, the way its scroll
    /// offset moves) from `at`, replacing a fling under way; a zero
    /// velocity starts none.
    ///
    /// # Errors
    ///
    /// [`FlingError`] when the velocity is not finite or `container` is
    /// not a scroll container; nothing changes.
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
