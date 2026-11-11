//! What an [`ElementsApp`]'s last frame handed the GPU.

use super::ElementsApp;
use crate::painter::{FrameCounts, ScenePainter};

impl ElementsApp {
    /// What the painter's last frame handed the GPU: see
    /// [`FrameCounts`]. All 0 before the first frame. A frame that fails
    /// before painting (an update refusal, a mount error) leaves the
    /// counts those of the frame before.
    #[must_use]
    pub fn frame_counts(&self) -> FrameCounts {
        self.painter
            .as_ref()
            .map(ScenePainter::frame_counts)
            .unwrap_or_default()
    }

    /// Render passes the last painted frame recorded into its encoder,
    /// atlas repacks and the pass budget's flushes included; 0 before
    /// the first. Not counted: a `gpu-surface` renderer's own passes.
    #[must_use]
    pub fn render_passes(&self) -> u64 {
        self.render_passes
    }
}
