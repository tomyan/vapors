//! What a scene render refuses, each naming the fix.

use std::fmt;
use vapors_scene3d::{CameraError, GeometryId, MaterialId, NodeId, SceneError, StoreError};

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
    /// The camera's world matrix scales, shears or mirrors
    /// (`CameraError::NotRigid`, which it holds).
    CameraNotRigid(CameraError),
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
    /// A mesh node names a material another `Meshes` issued.
    ForeignMaterial {
        /// The mesh node.
        node: NodeId,
        /// The material it names.
        material: MaterialId,
    },
    /// A mesh node names a material this renderer did not build at
    /// mount: given to its `Meshes` after the mount.
    MaterialMissing {
        /// The mesh node.
        node: NodeId,
        /// The material it names.
        material: MaterialId,
        /// How many materials were built.
        built: usize,
    },
    /// The active camera resolved to a node that is not a camera.
    ActiveCameraNotCamera(NodeId),
    /// A light node whose world matrix gives no finite position or a
    /// direction of zero length.
    LightDegenerate(NodeId),
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

impl From<CameraError> for FrameError {
    fn from(error: CameraError) -> Self {
        match error {
            CameraError::Store(error) => Self::Store(error),
            CameraError::NotCamera(node) => Self::ActiveCameraNotCamera(node),
            CameraError::Projection(error) => Self::Projection(error),
            CameraError::NotFinite(node) => Self::CameraNotFinite(node),
            not_rigid @ CameraError::NotRigid { .. } => Self::CameraNotRigid(not_rigid),
        }
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
            Self::ForeignMaterial { node, material } => write!(
                f,
                "{node} names {material}, which another Meshes issued: give it a material from this renderer's Meshes"
            ),
            Self::MaterialMissing {
                node,
                material,
                built,
            } => write!(
                f,
                "{node} names {material}, and this renderer built {built} materials when it mounted: give every material to the Meshes before mounting the renderer"
            ),
            Self::NotReady => write!(
                f,
                "SceneRenderer::render ran before mount and resize: mount it on a painter, which calls them first"
            ),
            Self::Store(error) => write!(f, "the scene: {error}"),
            Self::Projection(error) => write!(f, "the active camera: {error}"),
            Self::CameraNotRigid(error) => write!(f, "{error}"),
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
            Self::LightDegenerate(node) => write!(
                f,
                "light {node}'s world matrix gives no finite position or no direction: give it and its ancestors finite, non-zero scales (`cargo test -p vapors-scene3d-gpu lights`)"
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
