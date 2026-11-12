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
    /// A colour channel outside 0 to 1.
    ChannelOutOfRange {
        /// `red`, `green` or `blue`.
        channel: &'static str,
        /// Its value.
        value: f32,
    },
    /// Finite inputs whose matrix overflows to NaN or infinity.
    MatrixNotFinite {
        /// The constructor or method that built the matrix.
        what: &'static str,
        /// The inputs, formatted.
        inputs: String,
    },
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => {
                write!(f, "{field} is NaN or infinite: give a finite value")
            }
            Self::ZeroScale { axis } => fmt_zero_scale(f, *axis),
            Self::RotationNotUnit { length } => fmt_rotation(f, *length),
            Self::FieldOfView { radians } => fmt_field_of_view(f, *radians),
            Self::NearNotPositive { near } => fmt_near(f, *near),
            Self::FarNotBeyondNear { near, far } => fmt_far(f, *near, *far),
            Self::HeightNotPositive { height } => fmt_height(f, *height),
            Self::AspectNotPositive { aspect } => fmt_aspect(f, *aspect),
            Self::EyeAtTarget => fmt_eye_at_target(f),
            Self::UpParallel => fmt_up_parallel(f),
            Self::ChannelOutOfRange { channel, value } => fmt_channel(f, channel, *value),
            Self::MatrixNotFinite { what, inputs } => fmt_matrix(f, what, inputs),
        }
    }
}

fn fmt_zero_scale(f: &mut fmt::Formatter<'_>, axis: char) -> fmt::Result {
    write!(
        f,
        "Transform::scale.{axis} is 0, which flattens the node: give a non-zero scale, or remove the node to hide it"
    )
}

fn fmt_rotation(f: &mut fmt::Formatter<'_>, length: f32) -> fmt::Result {
    write!(
        f,
        "Transform::rotation has length {length}, not 1: build it with Quat::from_axis_angle or call normalize()"
    )
}

fn fmt_field_of_view(f: &mut fmt::Formatter<'_>, radians: f32) -> fmt::Result {
    write!(
        f,
        "Projection::perspective vertical_fov is {radians} radians: give a value strictly between 0 and pi"
    )
}

fn fmt_near(f: &mut fmt::Formatter<'_>, near: f32) -> fmt::Result {
    write!(
        f,
        "Projection::perspective near is {near}: give a distance greater than 0"
    )
}

fn fmt_far(f: &mut fmt::Formatter<'_>, near: f32, far: f32) -> fmt::Result {
    write!(
        f,
        "Projection far is {far}, not beyond near {near}: give far greater than near"
    )
}

fn fmt_height(f: &mut fmt::Formatter<'_>, height: f32) -> fmt::Result {
    write!(
        f,
        "Projection::orthographic height is {height}: give a height greater than 0"
    )
}

fn fmt_aspect(f: &mut fmt::Formatter<'_>, aspect: f32) -> fmt::Result {
    write!(
        f,
        "the viewport aspect ratio is {aspect}: an inactive (zero-sized) surface has no projection; give width / height of a painted one"
    )
}

fn fmt_eye_at_target(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "View::look_at eye equals target: move the eye or the target so the view has a direction"
    )
}

fn fmt_up_parallel(f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
        f,
        "View::look_at up is zero or parallel to the view direction (sine of the angle under 1e-6): give an up vector across it, such as Vec3::Y for a camera not looking straight down"
    )
}

fn fmt_channel(f: &mut fmt::Formatter<'_>, channel: &str, value: f32) -> fmt::Result {
    write!(
        f,
        "Colour::srgb {channel} is {value}: give an sRGB-encoded value from 0 to 1"
    )
}

fn fmt_matrix(f: &mut fmt::Formatter<'_>, what: &str, inputs: &str) -> fmt::Result {
    write!(
        f,
        "{what} gives a matrix with NaN or infinite elements for {inputs}: give values of moderate magnitude (distances under 1e19, fields of view and heights well above 1e-30)"
    )
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
