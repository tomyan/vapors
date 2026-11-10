//! The active camera: the one camera node holding the handle the scene
//! root names (`docs/plans/3d-2.md`, decision 2).

use crate::{CameraHandle, NodeId, NodeKind, Scene, StoreError};

impl Scene {
    /// Names the active camera by the handle its camera node holds;
    /// `None` names none. The name is checked by
    /// [`Scene::active_camera`], so a camera may be added after it.
    pub fn set_camera(&mut self, handle: Option<CameraHandle>) {
        self.camera = handle;
    }

    /// The camera node holding the handle the root names.
    ///
    /// # Errors
    ///
    /// [`StoreError::NoCamera`] when the root names none,
    /// [`StoreError::CameraUnresolved`] when no camera node holds it, and
    /// [`StoreError::CameraDuplicate`] when several do.
    pub fn active_camera(&self) -> Result<NodeId, StoreError> {
        let handle = self.camera.ok_or(StoreError::NoCamera)?;
        let holders: Vec<NodeId> = self
            .nodes()
            .filter(|(_, node)| matches!(node.kind, NodeKind::Camera { handle: held, .. } if held == handle))
            .map(|(id, _)| id)
            .collect();
        match holders.as_slice() {
            [] => Err(StoreError::CameraUnresolved(handle)),
            [one] => Ok(*one),
            _ => Err(StoreError::CameraDuplicate {
                handle,
                nodes: holders,
            }),
        }
    }
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
