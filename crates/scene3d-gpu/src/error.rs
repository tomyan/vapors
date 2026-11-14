//! What a scene render refuses, each naming the fix.

use std::fmt;
use vapors_scene3d::{GeometryId, NodeId, SceneError, StoreError};

/// A frame the scene renderer cannot draw.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameError {
    /// `render` before `mount` or `resize`.
    NotReady,
    /// The scene store refused: no, an unresolved or a duplicated
    /// active camera, among others.
    Store(StoreError),
    /// The camera's projection refused the backing's aspect.
    Projection(SceneError),
    /// The camera's world matrix has no finite inverse.
    CameraNotFinite(NodeId),
    /// A mesh node names geometry another `Meshes` issued.
    ForeignGeometry {
        /// The mesh node.
        node: NodeId,
        /// The geometry it names.
        geometry: GeometryId,
    },
    /// A mesh node names geometry its `Meshes` issued but no longer
    /// holds, and this renderer has not uploaded.
    GeometryMissing {
        /// The mesh node.
        node: NodeId,
        /// The geometry it names.
        geometry: GeometryId,
    },
    /// The active camera resolved to a node that is not a camera.
    ActiveCameraNotCamera(NodeId),
    /// The mesh uniform slots would need a buffer past the device's
    /// `max_buffer_size`.
    TooManySlots {
        /// Slots needed.
        slots: u32,
        /// Bytes per slot.
        stride: u32,
        /// The device's `max_buffer_size`, in bytes.
        limit: u64,
    },
}

impl From<StoreError> for FrameError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<SceneError> for FrameError {
    fn from(error: SceneError) -> Self {
        Self::Projection(error)
    }
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReady => write!(
                f,
                "SceneRenderer::render ran before mount and resize: mount it on a painter, which calls them first"
            ),
            Self::Store(error) => write!(f, "the scene: {error}"),
            Self::Projection(error) => write!(f, "the active camera: {error}"),
            Self::CameraNotFinite(node) => write!(
                f,
                "camera {node}'s world matrix has no finite inverse: give it and its ancestors transforms of moderate magnitude"
            ),
            Self::ForeignGeometry { node, geometry } => write!(
                f,
                "mesh {node} names {geometry}, which the renderer's Meshes did not issue: give it geometry from the Meshes passed to SceneRenderer::new"
            ),
            Self::GeometryMissing { node, geometry } => write!(
                f,
                "mesh {node} names {geometry}, which its Meshes has released: a broken invariant, since the node's GeometryHandle should keep its piece held; a defect in vapors-scene3d's Meshes or vapors-scene3d-gpu's release. Reproduce with `cargo test -p vapors-scene3d-gpu` and report it with the scene that produced it"
            ),
            Self::ActiveCameraNotCamera(node) => write!(
                f,
                "the active camera resolved to node {node}, which is not a camera: a vapors-scene3d defect; report it with the scene that produced it"
            ),
            Self::TooManySlots {
                slots,
                stride,
                limit,
            } => write!(
                f,
                "the scene needs {slots} mesh uniform slots of {stride} bytes, past this device's max_buffer_size of {limit} bytes: draw fewer mesh nodes in one scene"
            ),
        }
    }
}

impl std::error::Error for FrameError {}
