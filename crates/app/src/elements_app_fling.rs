//! Flinging one of the document's scroll containers
//! (`document_input_fling.rs`): each frame moves it on before the
//! update, and the app draws frames while it runs.

use vapors_dom::NodeId;
use vapors_platform::{Instant, Platform};

use super::ElementsApp;
use crate::document_input::FlingError;

impl ElementsApp {
    /// `container` flung at `velocity` (CSS px/s, the way its scroll
    /// offset moves) from `at`, replacing a fling under way; asks
    /// `platform` for a frame. A zero velocity starts none.
    ///
    /// # Errors
    ///
    /// [`FlingError`] when the velocity is not finite or `container` is
    /// not a scroll container; nothing changes.
    pub fn fling(
        &mut self,
        container: NodeId,
        velocity: [f32; 2],
        at: Instant,
        platform: &Platform,
    ) -> Result<(), FlingError> {
        self.input
            .fling(&self.document.borrow(), container, velocity, at)?;
        platform.request_redraw();
        Ok(())
    }
}
