//! What `pick` refuses, each naming the fix.

use std::fmt;

use crate::{CameraError, GeometryId, NodeId, StoreError};

/// Why nothing could be picked.
#[derive(Clone, Debug, PartialEq)]
pub enum PickError {
    /// The surface's size is not finite and positive on both axes.
    SizeNotPositive([f32; 2]),
    /// The point is not finite.
    PointNotFinite([f32; 2]),
    /// The active camera's view-projection was refused
    /// ([`crate::Scene::view_projection`]).
    Camera(CameraError),
    /// The camera's view-projection has no finite inverse.
    ViewNotInvertible(NodeId),
    /// A mesh's world matrix has no finite inverse.
    MeshNotInvertible(NodeId),
    /// A node read failed.
    Store(StoreError),
    /// A mesh names geometry its `Meshes` does not hold.
    GeometryNotHeld {
        /// The mesh node.
        node: NodeId,
        /// The geometry it names.
        geometry: GeometryId,
    },
}

impl From<StoreError> for PickError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<CameraError> for PickError {
    fn from(error: CameraError) -> Self {
        Self::Camera(error)
    }
}

impl fmt::Display for PickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeNotPositive(size) => write!(
                f,
                "nothing can be picked on a surface of size {size:?}: pass the surface's width and height in pixels, both finite and above 0"
            ),
            Self::PointNotFinite(point) => write!(
                f,
                "nothing can be picked at {point:?}: pass a finite point in surface pixels"
            ),
            Self::Camera(error) => write!(f, "nothing can be picked: {error}"),
            Self::ViewNotInvertible(camera) => write!(
                f,
                "nothing can be picked: camera {camera}'s view-projection has no finite inverse; give it and its ancestors transforms of moderate magnitude"
            ),
            Self::MeshNotInvertible(node) => write!(
                f,
                "mesh {node}'s world matrix has no finite inverse, so it cannot be picked: give it and its ancestors transforms of moderate magnitude"
            ),
            Self::Store(error) => write!(f, "nothing can be picked: {error}"),
            Self::GeometryNotHeld { node, geometry } => write!(
                f,
                "mesh {node} names {geometry}, which its Meshes does not hold: pick with the Meshes the scene's renderer draws from"
            ),
        }
    }
}

impl std::error::Error for PickError {}
