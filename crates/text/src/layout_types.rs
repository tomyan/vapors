//! What a layout is made of: options going in, lines, runs and placed
//! glyphs coming out.

use std::ops::Range;

use parlance::NormalizedCoord;
use parley::FontData;
use vapors_scene::Color;

/// Horizontal alignment of lines inside the layout width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// The writing direction's start edge.
    #[default]
    Start,
    /// The writing direction's end edge.
    End,
    /// Centred.
    Center,
    /// Stretched to the width, except the last line.
    Justify,
}

/// Width, alignment and display scale for a layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutOptions {
    /// Wrap width in logical pixels; `None` never wraps.
    pub width: Option<f32>,
    /// Line alignment inside the width.
    pub align: Align,
    /// Device pixels per logical pixel; the layout is in device pixels.
    pub scale: f32,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            width: None,
            align: Align::Start,
            scale: 1.0,
        }
    }
}

/// One line of a layout, in device pixels from the layout origin.
#[derive(Clone, Debug, PartialEq)]
pub struct LineInfo {
    /// The bytes of the text on this line, trailing whitespace included.
    pub text_range: Range<usize>,
    /// Baseline y, unsnapped.
    pub baseline: f32,
    /// The line box's top, device pixels.
    pub top: f32,
    /// The line box's height, device pixels.
    pub height: f32,
    /// Inked width.
    pub advance: f32,
}

/// One glyph of a [`GlyphRun`]. Positions are device pixels from the
/// layout origin; `y` is the baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// Glyph id in the run's font.
    pub id: u32,
    /// Left edge of the glyph's advance box.
    pub x: f32,
    /// Baseline.
    pub y: f32,
    /// Horizontal advance.
    pub advance: f32,
}

/// Consecutive glyphs in one font, size, variation and colour.
#[derive(Clone, Debug)]
pub struct GlyphRun {
    /// The font the glyphs are in.
    pub font: FontData,
    /// Font size in device pixels.
    pub size: f32,
    /// Variation coordinates the run was shaped with.
    pub coords: Vec<NormalizedCoord>,
    /// How far x moves right per pixel up: [`SYNTHETIC_SLANT`] when the
    /// run's italic is synthesised, else 0.
    ///
    /// [`SYNTHETIC_SLANT`]: crate::SYNTHETIC_SLANT
    pub skew: f32,
    /// Ink colour.
    pub color: Color,
    /// The glyphs, in visual order.
    pub glyphs: Vec<Glyph>,
}

/// A glyph with its run's font, size, variation and colour beside it.
#[derive(Clone, Copy, Debug)]
pub struct PlacedGlyph<'a> {
    /// The font the glyph is in.
    pub font: &'a FontData,
    /// Glyph id in that font.
    pub id: u32,
    /// Left edge of the glyph's advance box.
    pub x: f32,
    /// Baseline.
    pub y: f32,
    /// Horizontal advance.
    pub advance: f32,
    /// Font size in device pixels.
    pub size: f32,
    /// Variation coordinates the run was shaped with.
    pub coords: &'a [NormalizedCoord],
    /// The run's slant (see [`GlyphRun::skew`]).
    pub skew: f32,
    /// Ink colour.
    pub color: Color,
}
