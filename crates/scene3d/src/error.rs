//! What the scene refuses, each naming the field and the fix.

use std::fmt;

/// Input the scene cannot use.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneError {
    /// A value that is NaN or infinite; `field` names it.
    NonFinite {
        /// The field, such as `Transform::translation`.
        field: &'static str,
    },
    /// A scale component of zero, which collapses the node to a plane.
    ZeroScale {
        /// The axis: `x`, `y` or `z`.
        axis: char,
    },
    /// A rotation quaternion whose length is not 1.
    RotationNotUnit {
        /// Its length.
        length: f32,
    },
    /// A perspective field of view not strictly between 0 and pi.
    FieldOfView {
        /// The vertical field of view, in radians.
        radians: f32,
    },
    /// A perspective near plane at or behind the eye.
    NearNotPositive {
        /// The near distance.
        near: f32,
    },
    /// A far plane not beyond the near plane.
    FarNotBeyondNear {
        /// The near distance.
        near: f32,
        /// The far distance.
        far: f32,
    },
    /// An orthographic height of zero or less.
    HeightNotPositive {
        /// The height.
        height: f32,
    },
    /// A viewport aspect ratio (width / height) of zero or less.
    AspectNotPositive {
        /// The aspect ratio.
        aspect: f32,
    },
    /// A view whose eye is at its target, so it has no direction.
    EyeAtTarget,
    /// A view whose up vector is zero or parallel to its direction.
    UpParallel,
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => {
                write!(f, "{field} is NaN or infinite: give a finite value")
            }
            Self::ZeroScale { axis } => write!(
                f,
                "Transform::scale.{axis} is 0, which flattens the node: give a non-zero scale, or remove the node to hide it"
            ),
            Self::RotationNotUnit { length } => write!(
                f,
                "Transform::rotation has length {length}, not 1: build it with Quat::from_axis_angle or call normalize()"
            ),
            Self::FieldOfView { radians } => write!(
                f,
                "Projection::perspective vertical_fov is {radians} radians: give a value strictly between 0 and pi"
            ),
            Self::NearNotPositive { near } => write!(
                f,
                "Projection::perspective near is {near}: give a distance greater than 0"
            ),
            Self::FarNotBeyondNear { near, far } => write!(
                f,
                "Projection far is {far}, not beyond near {near}: give far greater than near"
            ),
            Self::HeightNotPositive { height } => write!(
                f,
                "Projection::orthographic height is {height}: give a height greater than 0"
            ),
            Self::AspectNotPositive { aspect } => write!(
                f,
                "the viewport aspect ratio is {aspect}: an inactive (zero-sized) surface has no projection; give width / height of a painted one"
            ),
            Self::EyeAtTarget => write!(
                f,
                "View::look_at eye equals target: move the eye or the target so the view has a direction"
            ),
            Self::UpParallel => write!(
                f,
                "View::look_at up is zero or parallel to the view direction: give an up vector across it, such as Vec3::Y for a camera not looking straight down"
            ),
        }
    }
}

impl std::error::Error for SceneError {}

/// Refuses a non-finite vector, naming `field`.
pub(crate) fn finite(field: &'static str, value: glam::Vec3) -> Result<(), SceneError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(SceneError::NonFinite { field })
    }
}

/// Refuses a non-finite number, naming `field`.
pub(crate) fn finite_scalar(field: &'static str, value: f32) -> Result<(), SceneError> {
    finite(field, glam::Vec3::splat(value))
}
