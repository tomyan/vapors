//! Colours as the scene carries them: non-premultiplied, encoded sRGB,
//! each channel a float in 0..=1.

/// A non-premultiplied sRGB colour with alpha, channels in 0..=1. The
/// default is transparent black, as CSS `transparent`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    /// Red, 0..=1.
    pub r: f32,
    /// Green, 0..=1.
    pub g: f32,
    /// Blue, 0..=1.
    pub b: f32,
    /// Alpha, 0..=1, where 1 is opaque.
    pub a: f32,
}

impl Color {
    /// Build a colour from 8-bit channels, as CSS hex colours give them.
    #[must_use]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a: f32::from(a) / 255.0,
        }
    }

    /// The nearest 8-bit channels; the inverse of [`Color::from_rgba8`].
    #[must_use]
    pub fn to_rgba8(self) -> [u8; 4] {
        [to_u8(self.r), to_u8(self.g), to_u8(self.b), to_u8(self.a)]
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255 first
fn to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_channels_round_trip_exactly() {
        // Given
        let channels = (12, 200, 34, 255);

        // When
        let color = Color::from_rgba8(channels.0, channels.1, channels.2, channels.3);

        // Then
        assert_eq!(color.to_rgba8(), [12, 200, 34, 255]);
    }

    #[test]
    fn channels_outside_range_clamp_when_converted() {
        // Given
        let color = Color {
            r: 1.5,
            g: -0.2,
            b: 0.5,
            a: 1.0,
        };

        // When
        let bytes = color.to_rgba8();

        // Then
        assert_eq!(bytes, [255, 0, 128, 255]);
    }
}
