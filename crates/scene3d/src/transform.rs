//! A node's local transform: translation, rotation, scale.

use crate::error::{SceneError, finite};
use glam::{Mat4, Quat, Vec3};

/// How far a rotation's length may be from 1 and still count as unit:
/// `Quat::from_axis_angle` of an `f32` angle lands within about 1e-7 of
/// it, and composing a few hundred rotations in `f32` stays within 1e-5.
const UNIT_TOLERANCE: f32 = 1e-4;

/// A validated local transform. Its matrix is `translation * rotation *
/// scale` (crate docs, conventions).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Transform {
    /// No translation, no rotation, unit scale.
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Refuses a non-finite value, a zero scale component, or a rotation
    /// whose length is not 1, naming the field.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`], [`SceneError::ZeroScale`] or
    /// [`SceneError::RotationNotUnit`].
    pub fn new(translation: Vec3, rotation: Quat, scale: Vec3) -> Result<Self, SceneError> {
        finite("Transform::translation", translation)?;
        finite("Transform::scale", scale)?;
        check_rotation(rotation)?;
        check_scale(scale)?;
        Ok(Self {
            translation,
            rotation,
            scale,
        })
    }

    /// A translation alone.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`].
    pub fn from_translation(translation: Vec3) -> Result<Self, SceneError> {
        Self::new(translation, Quat::IDENTITY, Vec3::ONE)
    }

    /// The local matrix: scale, then rotation, then translation.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

fn check_rotation(rotation: Quat) -> Result<(), SceneError> {
    if !rotation.is_finite() {
        return Err(SceneError::NonFinite {
            field: "Transform::rotation",
        });
    }
    let length = rotation.length();
    if (length - 1.0).abs() > UNIT_TOLERANCE {
        return Err(SceneError::RotationNotUnit { length });
    }
    Ok(())
}

fn check_scale(scale: Vec3) -> Result<(), SceneError> {
    for (axis, value) in ['x', 'y', 'z'].into_iter().zip(scale.to_array()) {
        if value == 0.0 {
            return Err(SceneError::ZeroScale { axis });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "transform_tests.rs"]
mod tests;
