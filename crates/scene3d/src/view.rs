//! Where a camera stands and what it looks at.

use crate::error::{SceneError, finite};
use glam::camera::rh::view;
use glam::{Mat4, Vec3};

/// A validated camera placement; its matrix takes world space to view
/// space, where the camera looks down -Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    eye: Vec3,
    target: Vec3,
    up: Vec3,
}

impl View {
    /// A camera at `eye` looking at `target`, with `up` giving its roll.
    /// Refuses a non-finite value, an eye at its target, and an up that
    /// is zero or parallel to the direction.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`], [`SceneError::EyeAtTarget`] or
    /// [`SceneError::UpParallel`].
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Result<Self, SceneError> {
        finite("View::eye", eye)?;
        finite("View::target", target)?;
        finite("View::up", up)?;
        let direction = target - eye;
        if direction.length_squared() == 0.0 {
            return Err(SceneError::EyeAtTarget);
        }
        if direction.cross(up).length_squared() == 0.0 {
            return Err(SceneError::UpParallel);
        }
        Ok(Self { eye, target, up })
    }

    /// World space to view space.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        view::look_at_mat4(self.eye, self.target, self.up)
    }
}
