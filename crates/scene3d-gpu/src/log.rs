//! What a [`SceneRenderer`](crate::SceneRenderer) reports about its own
//! lifetime, shared with whoever made it (as the racer's `TrackLog`):
//! that it was dropped, and with it every GPU resource it held (its
//! pipeline, depth texture, uniform slots, camera uniform and uploaded
//! geometry are its fields, freed when it drops).

use std::cell::Cell;

/// Shared by a renderer and the application through
/// [`SceneRenderer::log`](crate::SceneRenderer::log).
#[derive(Debug, Default)]
pub struct RendererLog {
    dropped: Cell<bool>,
}

impl RendererLog {
    /// Whether the renderer has been dropped: an unmounted surface's
    /// renderer is, once the caller of `ElementsApp::unmount` drops it.
    #[must_use]
    pub fn dropped(&self) -> bool {
        self.dropped.get()
    }

    pub(crate) fn drop_renderer(&self) {
        self.dropped.set(true);
    }
}
