//! The active camera: the one camera node holding the handle the scene
//! root names (`docs/plans/3d-2.md`, decision 2).

use crate::{CameraError, CameraHandle, Mat4, NodeId, NodeKind, Scene, StoreError};

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

    /// World space to clip space through the active camera: its
    /// projection for `aspect` (width / height), times the inverse of
    /// its world matrix. The one answer the renderer and
    /// [`crate::pick`] both use. The scene must be updated.
    ///
    /// # Errors
    ///
    /// [`CameraError`]: no resolvable camera, a refused aspect, or a
    /// matrix with an element that is not finite.
    pub fn view_projection(&self, aspect: f32) -> Result<Mat4, CameraError> {
        let camera = self.active_camera()?;
        let NodeKind::Camera { projection, .. } = self.kind(camera)? else {
            return Err(CameraError::NotCamera(camera));
        };
        let matrix = projection.matrix(aspect).map_err(CameraError::Projection)?;
        let view_projection = matrix * self.world(camera)?.inverse();
        if view_projection.is_finite() {
            Ok(view_projection)
        } else {
            Err(CameraError::NotFinite(camera))
        }
    }
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
