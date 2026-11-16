//! Why the active camera's view-projection cannot be had, each naming
//! the fix.

use crate::{NodeId, SceneError, StoreError};
use std::fmt;

/// Why [`crate::Scene::view_projection`] refused.
#[derive(Debug, Clone, PartialEq)]
pub enum CameraError {
    /// No, an unresolved or a duplicated active camera, or a node read
    /// failed.
    Store(StoreError),
    /// The active camera resolved to a node that is not a camera.
    NotCamera(NodeId),
    /// The camera's projection refused the aspect.
    Projection(SceneError),
    /// The view-projection has an element that is not finite: the
    /// camera's world matrix has no finite inverse.
    NotFinite(NodeId),
}

impl From<StoreError> for CameraError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(f, "the scene: {error}"),
            Self::NotCamera(node) => write!(
                f,
                "the active camera resolved to node {node}, which is not a camera: a vapors-scene3d defect; report it with the scene that produced it"
            ),
            Self::Projection(error) => write!(f, "the active camera: {error}"),
            Self::NotFinite(node) => write!(
                f,
                "camera {node}'s world matrix has no finite inverse: give it and its ancestors transforms of moderate magnitude"
            ),
        }
    }
}

impl std::error::Error for CameraError {}
