//! Perspective and orthographic projections, and projecting a point.

use crate::View;
use crate::error::{SceneError, finite_scalar};
use glam::camera::rh::proj::directx;
use glam::{Mat4, Vec3, Vec4Swizzles};
use std::f32::consts::PI;

/// A validated camera projection. Depth runs 0 at `near` to 1 at `far`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection {
    kind: Kind,
    near: f32,
    far: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Perspective { vertical_fov: f32 },
    Orthographic { height: f32 },
}

impl Projection {
    /// A perspective projection. Refuses a non-finite value, a field of
    /// view outside (0, pi), `near <= 0` and `far <= near`.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`], [`SceneError::FieldOfView`],
    /// [`SceneError::NearNotPositive`] or [`SceneError::FarNotBeyondNear`].
    pub fn perspective(vertical_fov: f32, near: f32, far: f32) -> Result<Self, SceneError> {
        finite_scalar("Projection::perspective vertical_fov", vertical_fov)?;
        if vertical_fov <= 0.0 || vertical_fov >= PI {
            return Err(SceneError::FieldOfView {
                radians: vertical_fov,
            });
        }
        finite_scalar("Projection::perspective near", near)?;
        if near <= 0.0 {
            return Err(SceneError::NearNotPositive { near });
        }
        Self::with_depth(Kind::Perspective { vertical_fov }, near, far)
    }

    /// An orthographic projection `height` world units tall; its width
    /// follows the viewport's aspect. `near` may be zero or negative.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`], [`SceneError::HeightNotPositive`] or
    /// [`SceneError::FarNotBeyondNear`].
    pub fn orthographic(height: f32, near: f32, far: f32) -> Result<Self, SceneError> {
        finite_scalar("Projection::orthographic height", height)?;
        if height <= 0.0 {
            return Err(SceneError::HeightNotPositive { height });
        }
        finite_scalar("Projection::orthographic near", near)?;
        Self::with_depth(Kind::Orthographic { height }, near, far)
    }

    fn with_depth(kind: Kind, near: f32, far: f32) -> Result<Self, SceneError> {
        finite_scalar("Projection far", far)?;
        if far <= near {
            return Err(SceneError::FarNotBeyondNear { near, far });
        }
        Ok(Self { kind, near, far })
    }

    /// View space to clip space for a viewport of `aspect` (width /
    /// height).
    ///
    /// # Errors
    ///
    /// [`SceneError::AspectNotPositive`] or [`SceneError::NonFinite`].
    pub fn matrix(&self, aspect: f32) -> Result<Mat4, SceneError> {
        finite_scalar("the viewport aspect ratio", aspect)?;
        if aspect <= 0.0 {
            return Err(SceneError::AspectNotPositive { aspect });
        }
        Ok(match self.kind {
            Kind::Perspective { vertical_fov } => {
                directx::perspective(vertical_fov, aspect, self.near, self.far)
            }
            Kind::Orthographic { height } => {
                let (half_h, half_w) = (height / 2.0, height / 2.0 * aspect);
                directx::orthographic(-half_w, half_w, -half_h, half_h, self.near, self.far)
            }
        })
    }
}

/// World space to clip space: `projection * view`. Every scene draw
/// takes its camera from this one matrix, ground and objects alike.
///
/// # Errors
///
/// As [`Projection::matrix`].
pub fn view_projection(
    projection: &Projection,
    view: &View,
    aspect: f32,
) -> Result<Mat4, SceneError> {
    Ok(projection.matrix(aspect)? * view.matrix())
}

/// A world point through `view_projection` to normalised device
/// coordinates: x and y from -1 to 1 (+Y up), z the depth from 0 to 1.
/// `None` when the point is at or behind the eye plane, which has no
/// position on screen; a point beyond the frustum's sides, near or far
/// is returned as is, outside those ranges.
#[must_use]
pub fn project(view_projection: Mat4, point: Vec3) -> Option<Vec3> {
    let clip = view_projection * point.extend(1.0);
    (clip.w > 0.0).then(|| clip.xyz() / clip.w)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
