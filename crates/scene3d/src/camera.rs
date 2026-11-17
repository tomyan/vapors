//! The active camera: the one camera node holding the handle the scene
//! root names (`docs/plans/3d-2.md`, decision 2).

use crate::{CameraError, CameraHandle, Mat4, NodeId, NodeKind, Projection, Scene, StoreError};

/// How far a rigid camera's axis lengths and their dot products may be
/// from 1 and 0: f32 composition of rotations stays within about 1e-6.
const RIGID_TOLERANCE: f32 = 1e-4;

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
        let world = self.world(camera)?;
        rigid(camera, world)?;
        let view_projection = matrix * world.inverse();
        if view_projection.is_finite() {
            Ok(view_projection)
        } else {
            Err(CameraError::NotFinite(camera))
        }
    }

    /// Sets camera `id`'s lens; whether it changed. The renderer writes
    /// the camera's uniform when the view-projection changes.
    ///
    /// # Errors
    ///
    /// [`StoreError::Stale`], or [`StoreError::NotCamera`] for a group or
    /// mesh.
    pub fn set_projection(&mut self, id: NodeId, lens: Projection) -> Result<bool, StoreError> {
        let node = self.node_mut(id)?;
        let NodeKind::Camera { projection, .. } = &mut node.kind else {
            return Err(StoreError::NotCamera(id, "set_projection"));
        };
        let changed = *projection != lens;
        *projection = lens;
        Ok(changed)
    }
}

/// `world`, a camera's, refused unless it is a rotation and a
/// translation (`CameraError::NotRigid`).
fn rigid(node: NodeId, world: Mat4) -> Result<(), CameraError> {
    let axes = [world.x_axis, world.y_axis, world.z_axis].map(glam::Vec4::truncate);
    let lengths = axes.map(glam::Vec3::length);
    let unit = lengths
        .iter()
        .all(|length| (length - 1.0).abs() <= RIGID_TOLERANCE);
    let square = [(0, 1), (1, 2), (0, 2)]
        .iter()
        .all(|&(a, b)| axes[a].dot(axes[b]).abs() <= RIGID_TOLERANCE);
    let mirrored = world.determinant() < 0.0;
    if unit && square && !mirrored {
        return Ok(());
    }
    Err(CameraError::NotRigid {
        node,
        lengths,
        mirrored,
    })
}

#[cfg(test)]
#[path = "camera_tests.rs"]
mod tests;
