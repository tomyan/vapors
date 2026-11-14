//! The page's transforms: places, the camera's view, and the group's
//! turn following the page's time signal.

use std::time::Duration;

use vapors_scene3d::{Quat, Transform, Vec3};
use vapors_signals::{Effect, Signal};

use crate::schedule::turn;

/// The group's turn at the page's time, following it.
///
/// # Panics
/// Never: a rotation about +Y with unit scale is a valid transform.
#[must_use]
pub fn turning(time: Signal<Duration>) -> Signal<Transform> {
    let at = |time| {
        let rotation = Quat::from_rotation_y(turn(time));
        Transform::new(Vec3::ZERO, rotation, Vec3::ONE).expect("a turn about +Y")
    };
    let transform = Signal::new(at(time.get_untracked()));
    Effect::new(move || transform.set(at(time.get())));
    transform
}

/// A node at `(x, y, z)`.
///
/// # Panics
/// For a non-finite coordinate; the page's are literals.
#[must_use]
pub fn at(x: f32, y: f32, z: f32) -> Signal<Transform> {
    Signal::new(Transform::from_translation(Vec3::new(x, y, z)).expect("a finite place"))
}

/// The camera 8 back and 3 up, tilted down at the cubes.
///
/// # Panics
/// Never: a fixed, finite view.
#[must_use]
pub fn looking() -> Signal<Transform> {
    let tilt = Quat::from_rotation_x(-0.35);
    let view = Transform::new(Vec3::new(0.0, 3.0, 8.0), tilt, Vec3::ONE).expect("a fixed view");
    Signal::new(view)
}
