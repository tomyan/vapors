//! Where things are drawn. One board square is one world unit: column
//! `x` is at world x, row `r` at world z = r, so the far bank (row 0) is
//! furthest from the camera, which sits behind the start and above it,
//! tilted down at the lanes. +Y is up.

use std::f32::consts::FRAC_PI_2;

use vapors_scene3d::{Projection, Quat, Transform, Vec3};

use crate::game::{HEIGHT, WIDTH};

/// A node at `(x, y, z)`, turned `turn` radians about +Y, scaled by
/// `size` (width, height, depth).
///
/// # Panics
/// For a non-finite place or size.
#[must_use]
pub fn at(place: (f32, f32, f32), turn: f32, size: (f32, f32, f32)) -> Transform {
    let (x, y, z) = place;
    let scale = Vec3::new(size.0, size.1, size.2);
    Transform::new(Vec3::new(x, y, z), Quat::from_rotation_y(turn), scale).expect("a finite place")
}

/// A cylinder laid along X (a log), `length` long and `width` across.
///
/// # Panics
/// For a non-finite place or size.
#[must_use]
pub fn lying(place: (f32, f32, f32), length: f32, width: f32) -> Transform {
    let (x, y, z) = place;
    let scale = Vec3::new(width, length, width);
    Transform::new(Vec3::new(x, y, z), Quat::from_rotation_z(FRAC_PI_2), scale)
        .expect("a finite place")
}

/// The camera behind the start row and above it, tilted down.
///
/// # Panics
/// Never: a fixed, finite place.
#[must_use]
pub fn behind() -> Transform {
    let eye = Vec3::new((WIDTH - 1) as f32 / 2.0, 6.5, HEIGHT as f32 + 3.5);
    Transform::new(eye, Quat::from_rotation_x(-0.62), Vec3::ONE).expect("a fixed place")
}

/// A perspective lens, 45 degrees from top to bottom.
///
/// # Errors
/// Never for these fixed numbers.
pub fn lens() -> Result<Projection, String> {
    Projection::perspective(0.8, 0.1, 100.0).map_err(|e| e.to_string())
}

/// The turn about +Y that points a model's front (+Z) along a lane
/// moving at `speed`: towards +X when positive, -X when negative.
#[must_use]
pub fn heading(speed: f32) -> f32 {
    if speed < 0.0 { -FRAC_PI_2 } else { FRAC_PI_2 }
}
