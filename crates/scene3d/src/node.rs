//! A node: what it is, where it sits, and reading it from the scene.

use crate::{CameraHandle, Colour, GeometryId, NodeId, Projection, Scene, StoreError, Transform};
use glam::Mat4;

/// What a node is.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    /// Holds children; draws nothing.
    Group,
    /// Draws geometry in one colour.
    Mesh {
        /// The triangles, given to the scene renderer's [`crate::Meshes`].
        geometry: GeometryId,
        /// Drawn unlit.
        colour: Colour,
    },
    /// A camera the scene root can name by `handle`.
    Camera {
        /// The name the root points at.
        handle: CameraHandle,
        /// How it projects.
        projection: Projection,
    },
}

#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) kind: NodeKind,
    pub(crate) local: Transform,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) world: Mat4,
    /// How many times `world` has been recomputed.
    pub(crate) world_version: u64,
    pub(crate) dirty: bool,
}

impl Scene {
    /// What `id` is.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn kind(&self, id: NodeId) -> Result<&NodeKind, StoreError> {
        Ok(&self.node(id)?.kind)
    }

    /// `id`'s parent; `None` for the root.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn parent(&self, id: NodeId) -> Result<Option<NodeId>, StoreError> {
        Ok(self.node(id)?.parent)
    }

    /// `id`'s children, in the order they were added or moved in.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn children(&self, id: NodeId) -> Result<&[NodeId], StoreError> {
        Ok(&self.node(id)?.children)
    }
}
