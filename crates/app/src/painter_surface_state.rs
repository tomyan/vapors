//! What the painter holds about one mounted surface, for a failing
//! check to print (`ScenePainter::surface_states`).

use super::ScenePainter;

/// One mounted surface's flags on the painter.
#[derive(Clone, Debug, PartialEq, Eq)]
// the painter's own flags, one field each
#[allow(clippy::struct_excessive_bools)]
pub struct SurfaceState {
    /// The surface's name.
    pub name: String,
    /// Mounted, and no frame has ended since.
    pub fresh: bool,
    /// Invalidated since its last render.
    pub stale: bool,
    /// Rendered on every frame that paints it.
    pub animating: bool,
    /// Holds a backing texture: the last frame painted it.
    pub backing: bool,
    /// A GPU error scope of its calls is unresolved.
    pub waiting: bool,
}

impl ScenePainter {
    /// Each mounted surface's state, by name.
    #[must_use]
    pub fn surface_states(&self) -> Vec<SurfaceState> {
        self.surfaces.states()
    }
}
