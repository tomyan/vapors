//! A node's appearance: what the renderer writes beside its world
//! matrix.

use crate::{Colour, NodeId, NodeKind, Scene, StoreError};

impl Scene {
    /// Sets a mesh's colour and whether it is drawn (its anchor's
    /// `color` and `visibility`, docs/plans/css-for-3d.md C3); whether
    /// either changed. Another colour advances the mesh's
    /// [`Scene::world_version`], so the renderer writes its uniform; a
    /// change of visibility alone does not: a hidden mesh is not drawn,
    /// and shown again its slot still holds what it held. Opaque only
    /// until the blended pass (C5).
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::NotMesh`] for a group or
    /// camera.
    pub fn set_appearance(
        &mut self,
        id: NodeId,
        colour: Colour,
        visible: bool,
    ) -> Result<bool, StoreError> {
        let node = self.node_mut(id)?;
        let NodeKind::Mesh { colour: held, .. } = &mut node.kind else {
            return Err(StoreError::NotMesh(id));
        };
        let recoloured = *held != colour;
        let changed = recoloured || node.visible != visible;
        *held = colour;
        node.visible = visible;
        node.world_version += u64::from(recoloured);
        Ok(changed)
    }

    /// Whether `id` is drawn: false after
    /// [`Scene::set_appearance`] hid it.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`].
    pub fn visible(&self, id: NodeId) -> Result<bool, StoreError> {
        Ok(self.node(id)?.visible)
    }
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
