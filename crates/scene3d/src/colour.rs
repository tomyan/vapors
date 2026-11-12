//! A mesh's colour until materials come (3D lane slice 3).

use crate::SceneError;
use crate::error::finite_scalar;

/// An opaque colour as sRGB-encoded channels from 0 to 1, the values the
/// surface backing holds (`vapors_app::SURFACE_FORMAT`). The scene draws
/// it unlit, so it lands in the backing as given; no channel is
/// converted to linear and back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colour {
    rgb: [f32; 3],
}

impl Colour {
    /// Every channel 0: CSS's initial `color`.
    pub const BLACK: Self = Self { rgb: [0.0; 3] };

    /// sRGB-encoded red, green and blue, each from 0 to 1.
    ///
    /// # Errors
    ///
    /// [`SceneError::NonFinite`] or [`SceneError::ChannelOutOfRange`].
    pub fn srgb(red: f32, green: f32, blue: f32) -> Result<Self, SceneError> {
        let named = [
            ("red", "Colour::srgb red", red),
            ("green", "Colour::srgb green", green),
            ("blue", "Colour::srgb blue", blue),
        ];
        for (channel, field, value) in named {
            finite_scalar(field, value)?;
            if !(0.0..=1.0).contains(&value) {
                return Err(SceneError::ChannelOutOfRange { channel, value });
            }
        }
        Ok(Self {
            rgb: [red, green, blue],
        })
    }

    /// The channels with alpha 1: `[red, green, blue, 1]`.
    #[must_use]
    pub fn rgba(&self) -> [f32; 4] {
        let [r, g, b] = self.rgb;
        [r, g, b, 1.0]
    }
}

#[cfg(test)]
#[path = "colour_tests.rs"]
mod tests;
