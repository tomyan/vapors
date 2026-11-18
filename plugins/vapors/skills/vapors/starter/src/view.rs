//! Where things are drawn. +X is right, +Y up, +Z towards the viewer; a
//! camera looks down its own -Z.

use vapors_scene3d::{Projection, Quat, Transform, Vec3};

/// The box, turned `angle` radians about +Y.
///
/// # Panics
/// For a non-finite angle.
#[must_use]
pub fn turned(angle: f32) -> Transform {
    Transform::new(Vec3::ZERO, Quat::from_rotation_y(angle), Vec3::ONE).expect("a finite turn")
}

/// The camera in front of the box and above it, tilted down at it.
///
/// # Panics
/// Never: a fixed, finite place.
#[must_use]
pub fn camera() -> Transform {
    let eye = Vec3::new(0.0, 1.4, 3.0);
    Transform::new(eye, Quat::from_rotation_x(-0.42), Vec3::ONE).expect("a fixed place")
}

/// A perspective lens, 46 degrees from top to bottom.
///
/// # Errors
/// Never for these fixed numbers.
pub fn lens() -> Result<Projection, String> {
    Projection::perspective(0.8, 0.1, 100.0).map_err(|e| e.to_string())
}
