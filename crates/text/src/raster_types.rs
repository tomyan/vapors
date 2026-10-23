//! What goes into the rasteriser and what comes out: the request, the
//! coverage bitmap, the hinting choice, and the failure.

use std::fmt;

use parlance::NormalizedCoord;
use parley::FontData;

/// Horizontal subpixel positions a glyph is rasterised at.
pub const SUBPIXEL_STEPS: u8 = 4;

/// Whether outlines are snapped to the pixel grid before rasterising.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Hinting {
    /// Outlines as designed.
    None,
    /// Vertical-only snapping (stems and x-height to whole pixels).
    Light,
}

impl Hinting {
    /// Light below 2x, none at 2x and above, as macOS does.
    #[must_use]
    pub fn for_scale(scale: f32) -> Self {
        if scale < 2.0 { Self::Light } else { Self::None }
    }
}

/// What to rasterise.
#[derive(Clone, Copy, Debug)]
pub struct GlyphRequest<'a> {
    /// The font.
    pub font: &'a FontData,
    /// Glyph id in the font.
    pub id: u32,
    /// Font size in device pixels.
    pub size: f32,
    /// Variation coordinates.
    pub coords: &'a [NormalizedCoord],
    /// Horizontal offset in steps of `1 / SUBPIXEL_STEPS` pixel.
    pub subpixel: u8,
    /// Hinting mode.
    pub hinting: Hinting,
    /// Slant: x moves this many pixels right per pixel up (0 upright).
    pub skew: f32,
}

/// A rasterised glyph: 8-bit coverage, placed relative to the glyph
/// origin on the baseline. `top` is negative for ink above the baseline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    /// Left edge from the origin, pixels.
    pub left: i32,
    /// Top edge from the baseline, pixels; negative is up.
    pub top: i32,
    /// Bitmap width.
    pub width: u32,
    /// Bitmap height.
    pub height: u32,
    /// `width * height` coverage bytes, rows from the top.
    pub data: Vec<u8>,
    /// True when the glyph could not be drawn and this is `.notdef`.
    pub notdef: bool,
}

impl Coverage {
    /// True for glyphs with no ink, such as a space.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Why nothing could be rasterised, not even `.notdef`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RasterError {
    /// The font data does not parse.
    NotAFont {
        /// Blob id of the data.
        font: u64,
        /// Face index.
        index: u32,
    },
    /// `.notdef` itself could not be drawn.
    NotdefFailed {
        /// Blob id of the font.
        font: u64,
        /// Why.
        reason: String,
    },
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAFont { font, index } => write!(
                f,
                "font blob {font} face {index} is not a font skrifa can read. \
                 Fix: register a TrueType or OpenType file; check what was passed to FontSystem."
            ),
            Self::NotdefFailed { font, reason } => write!(
                f,
                "font blob {font}: .notdef cannot be drawn ({reason}). \
                 Fix: the font has no usable glyph 0; use a different font file."
            ),
        }
    }
}

impl std::error::Error for RasterError {}

pub(crate) fn subpixel_offset(step: u8) -> f32 {
    f32::from(step % SUBPIXEL_STEPS) / f32::from(SUBPIXEL_STEPS)
}

/// Split an x position into the pixel to place at and the subpixel step
/// to rasterise with, rounding to the nearest step.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // rounded, then 0..4
pub fn subpixel_of(x: f32) -> (i32, u8) {
    let steps = i32::from(SUBPIXEL_STEPS);
    let quantised = (x * f32::from(SUBPIXEL_STEPS)).round() as i32;
    (
        quantised.div_euclid(steps),
        quantised.rem_euclid(steps) as u8,
    )
}
