//! How a mesh is shaded: its colour as it is, or lit by the scene's
//! lights (`docs/plans/lighting.md`, LT3).

use crate::{NodeId, NodeKind, Scene, StoreError};

/// A mesh's shading, a closed set (the first two material kinds, parity plan M1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shading {
    /// Its colour times the vertex shade, as it has always drawn.
    #[default]
    Unlit,
    /// Ambient and Lambert diffuse over the
    /// scene's lights, in linear light, times the vertex shade. Its
    /// geometry needs normals (`Meshes::lit_geometry`).
    Lit,
}

impl Scene {
    /// Sets mesh `id`'s shading.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::NotMesh`] for a group,
    /// camera or light.
    pub fn set_shading(&mut self, id: NodeId, shading: Shading) -> Result<(), StoreError> {
        let node = self.node_mut(id)?;
        if !matches!(node.kind, NodeKind::Mesh { .. }) {
            return Err(StoreError::NotMesh(id, "set_shading"));
        }
        node.shading = shading;
        Ok(())
    }

    /// `id`'s shading: [`Shading::Unlit`] unless set, and for a node
    /// that is not a mesh.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn shading(&self, id: NodeId) -> Result<Shading, StoreError> {
        Ok(self.node(id)?.shading)
    }
}

#[cfg(test)]
#[path = "shading_tests.rs"]
mod tests;
