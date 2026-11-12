//! A node's appearance: what the renderer writes beside its world
//! matrix.

use crate::{Colour, NodeId, NodeKind, Scene, StoreError};

impl Scene {
    /// Sets a mesh's colour. It advances the mesh's
    /// [`Scene::world_version`], which the renderer compares to decide
    /// what to write, and recomputes no world matrix.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::NotMesh`] for a group or
    /// camera.
    pub fn set_colour(&mut self, id: NodeId, colour: Colour) -> Result<(), StoreError> {
        let node = self.node_mut(id)?;
        let NodeKind::Mesh { colour: held, .. } = &mut node.kind else {
            return Err(StoreError::NotMesh(id));
        };
        *held = colour;
        node.world_version += 1;
        Ok(())
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
