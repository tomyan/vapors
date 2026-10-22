//! What the caller supplies to the cascade.

use crate::value::ColorScheme;

/// What the caller knows that `@media` and the relative units need.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Environment {
    /// Viewport width in px.
    pub viewport_width: f64,
    /// Viewport height in px.
    pub viewport_height: f64,
    /// The colour scheme for `light-dark()` and `prefers-color-scheme`.
    pub color_scheme: ColorScheme,
    /// `prefers-reduced-motion: reduce` when true.
    pub reduced_motion: bool,
    /// Device pixels per CSS pixel, for `resolution`.
    pub resolution: f64,
    /// The font size the root inherits from (the user agent default).
    pub root_font_size: f64,
    /// The advance of `0` as a fraction of the font size, for `ch`, until
    /// the text layer supplies the real measurement.
    pub zero_advance_em: f64,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            viewport_width: 0.0,
            viewport_height: 0.0,
            color_scheme: ColorScheme::Light,
            reduced_motion: false,
            resolution: 1.0,
            root_font_size: 16.0,
            zero_advance_em: 0.5,
        }
    }
}
