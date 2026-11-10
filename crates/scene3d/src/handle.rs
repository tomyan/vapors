//! Names for scene nodes and cameras.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// A node in one [`crate::Scene`]. The generation changes each time a
/// slot is reused, so the id of a removed node is refused as stale
/// rather than naming whichever node took its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node {}v{}", self.index, self.generation)
    }
}

/// The name a camera node holds and the scene root points at to make
/// that camera active (`docs/plans/3d-2.md`, decision 2). Each call to
/// [`CameraHandle::new`] gives a handle no other call in the process
/// gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CameraHandle(u64);

static NEXT_CAMERA: AtomicU64 = AtomicU64::new(0);

impl CameraHandle {
    /// A handle unique in this process.
    #[must_use]
    pub fn new() -> Self {
        Self(NEXT_CAMERA.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for CameraHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CameraHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "camera handle {}", self.0)
    }
}
