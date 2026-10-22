//! Colours as the cascade keeps them: sRGB, `currentColor`, or a
//! `light-dark()` pair resolved at computed-value time.

use vapors_scene::Color as Rgba;

use super::color_space;

/// The colour scheme `light-dark()` and `prefers-color-scheme` follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    /// Light.
    Light,
    /// Dark.
    Dark,
}

/// A specified colour.
#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    /// An sRGB colour with alpha.
    Rgba(Rgba),
    /// The element's `color` property (or, for `color` itself, the
    /// parent's).
    CurrentColor,
    /// `light-dark(light, dark)`.
    LightDark(Box<Color>, Box<Color>),
}

impl Color {
    /// Fully transparent black, the `transparent` keyword.
    pub const TRANSPARENT: Self = Self::Rgba(Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    });

    /// The colour as sRGB under `scheme`, with `current` for `currentColor`.
    #[must_use]
    pub fn resolve(&self, scheme: ColorScheme, current: Rgba) -> Rgba {
        match self {
            Self::Rgba(c) => *c,
            Self::CurrentColor => current,
            Self::LightDark(light, dark) => match scheme {
                ColorScheme::Light => light.resolve(scheme, current),
                ColorScheme::Dark => dark.resolve(scheme, current),
            },
        }
    }

    /// The colour in Oklab (lightness, a, b) with alpha, for perceptual
    /// interpolation.
    #[must_use]
    pub fn to_oklab(rgba: Rgba) -> [f64; 4] {
        let (l, a, b) = color_space::linear_to_oklab(
            color_space::decode(f64::from(rgba.r)),
            color_space::decode(f64::from(rgba.g)),
            color_space::decode(f64::from(rgba.b)),
        );
        [l, a, b, f64::from(rgba.a)]
    }

    /// An sRGB colour from Oklab lightness, a, b and alpha.
    #[must_use]
    pub fn from_oklab(lab: [f64; 4]) -> Rgba {
        let (r, g, b) = color_space::oklab_to_linear(lab[0], lab[1], lab[2]);
        Rgba {
            r: unit(color_space::encode(r)),
            g: unit(color_space::encode(g)),
            b: unit(color_space::encode(b)),
            a: unit(lab[3]),
        }
    }

    /// An opaque colour from 0..=255 channels given as floats, clamped.
    pub(super) fn from_channels(r: f64, g: f64, b: f64, alpha: f64) -> Self {
        Self::Rgba(Rgba {
            r: unit(r / 255.0),
            g: unit(g / 255.0),
            b: unit(b / 255.0),
            a: unit(alpha),
        })
    }
}

#[allow(clippy::cast_possible_truncation)] // f32 precision is what the scene carries
fn unit(v: f64) -> f32 {
    v.clamp(0.0, 1.0) as f32
}
