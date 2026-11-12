//! Where a camera stands and what it looks at.

use crate::error::{SceneError, finite};
use glam::camera::rh::view;
use glam::{Mat4, Vec3};

/// The smallest `|direction x up|`, both normalised (the sine of the
/// angle between them), that counts as an up across the direction. At
/// 1e-6 the angle is about 0.00006 degrees, where `f32` roll is
/// noise; an exact zero test accepts ups that produce a garbage roll.
const MIN_UP_SINE: f32 = 1e-6;

/// A validated camera placement; its matrix takes world space to view
/// space, where the camera looks down -Z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    matrix: Mat4,
}

impl View {
    /// A camera at `eye` looking at `target`, with `up` giving its roll
    /// (any length). Refuses a non-finite value, an eye at its target,
    /// an up that is zero or within a sine of 1e-6 of the direction, and
    /// inputs whose matrix is not finite (a direction of 1e19 or more).
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`], [`SceneError::EyeAtTarget`],
    /// [`SceneError::UpParallel`] or [`SceneError::MatrixNotFinite`].
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Result<Self, SceneError> {
        finite("View::eye", eye)?;
        finite("View::target", target)?;
        finite("View::up", up)?;
        let direction = unit(target - eye).ok_or(SceneError::EyeAtTarget)?;
        let up = unit(up).ok_or(SceneError::UpParallel)?;
        if direction.cross(up).length() < MIN_UP_SINE {
            return Err(SceneError::UpParallel);
        }
        let matrix = view::look_at_mat4(eye, target, up);
        if !matrix.is_finite() {
            return Err(SceneError::MatrixNotFinite {
                what: "View::look_at",
                inputs: format!("eye {eye}, target {target}, up {up}"),
            });
        }
        Ok(Self { matrix })
    }

    /// World space to view space.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        self.matrix
    }
}

/// `vector` scaled to length 1, or `None` for the zero vector. Divides
/// by the largest component first, so a length that overflows `f32`
/// does not read as zero.
fn unit(vector: Vec3) -> Option<Vec3> {
    let largest = vector.abs().max_element();
    (largest > 0.0).then(|| (vector / largest).normalize())
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
