//! One glyph to coverage on the CPU: skrifa's outline and hinting, then
//! zeno's rasteriser, at one of [`SUBPIXEL_STEPS`] horizontal offsets.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;

use parlance::NormalizedCoord;
use parley::FontData;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, HintingInstance, OutlineGlyphCollection};
use skrifa::{GlyphId, MetadataProvider};

use crate::pen::PathCommands;

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

#[derive(Hash, PartialEq, Eq)]
struct HintKey {
    font: u64,
    index: u32,
    size_bits: u32,
    coords: Vec<i16>,
}

/// Rasterises glyphs, keeping hinting state per font and size.
#[derive(Default)]
pub struct Rasterizer {
    hinters: HashMap<HintKey, HintingInstance>,
}

impl Rasterizer {
    /// No state yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rasterise `request`. A glyph that cannot be drawn becomes `.notdef`
    /// and is logged with the font and glyph id; it is never skipped.
    ///
    /// # Errors
    ///
    /// [`RasterError`] when the font does not parse or `.notdef` itself
    /// cannot be drawn.
    pub fn rasterize(&mut self, request: &GlyphRequest<'_>) -> Result<Coverage, RasterError> {
        let font_id = request.font.data.id();
        let font = skrifa::FontRef::from_index(request.font.data.as_ref(), request.font.index)
            .map_err(|_| RasterError::NotAFont {
                font: font_id,
                index: request.font.index,
            })?;
        let outlines = font.outline_glyphs();
        let coords = skrifa_coords(request.coords);
        match self.draw(&outlines, request, &coords, request.id) {
            Ok(coverage) => Ok(coverage),
            Err(reason) => {
                log::error!(
                    "font {font_id} glyph {}: {reason}; drawing .notdef",
                    request.id
                );
                self.draw(&outlines, request, &coords, 0)
                    .map(|c| Coverage { notdef: true, ..c })
                    .map_err(|reason| RasterError::NotdefFailed {
                        font: font_id,
                        reason,
                    })
            }
        }
    }

    fn draw(
        &mut self,
        outlines: &OutlineGlyphCollection<'_>,
        request: &GlyphRequest<'_>,
        coords: &[skrifa::instance::NormalizedCoord],
        id: u32,
    ) -> Result<Coverage, String> {
        let outline = outlines
            .get(GlyphId::new(id))
            .ok_or_else(|| "no outline".to_owned())?;
        let size = Size::new(request.size);
        let location = LocationRef::new(coords);
        let mut path = PathCommands::default();
        match request.hinting {
            Hinting::None => outline.draw(DrawSettings::unhinted(size, location), &mut path),
            Hinting::Light => {
                let hinter = self.hinter(outlines, request, coords)?;
                outline.draw(DrawSettings::hinted(hinter, false), &mut path)
            }
        }
        .map_err(|e| e.to_string())?;
        Ok(path.rasterize(subpixel_offset(request.subpixel)))
    }

    fn hinter(
        &mut self,
        outlines: &OutlineGlyphCollection<'_>,
        request: &GlyphRequest<'_>,
        coords: &[skrifa::instance::NormalizedCoord],
    ) -> Result<&HintingInstance, String> {
        let key = HintKey {
            font: request.font.data.id(),
            index: request.font.index,
            size_bits: request.size.to_bits(),
            coords: request.coords.iter().map(|c| c.to_bits()).collect(),
        };
        match self.hinters.entry(key) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let instance = HintingInstance::new(
                    outlines,
                    Size::new(request.size),
                    LocationRef::new(coords),
                    light_hinting(),
                )
                .map_err(|e| e.to_string())?;
                Ok(entry.insert(instance))
            }
        }
    }
}

fn light_hinting() -> skrifa::outline::HintingOptions {
    use skrifa::outline::{Engine, SmoothMode, Target};
    skrifa::outline::HintingOptions {
        engine: Engine::Auto(None),
        target: Target::Smooth {
            mode: SmoothMode::Light,
            symmetric_rendering: false,
            preserve_linear_metrics: true,
        },
    }
}

fn skrifa_coords(coords: &[NormalizedCoord]) -> Vec<skrifa::instance::NormalizedCoord> {
    coords
        .iter()
        .map(|c| skrifa::instance::NormalizedCoord::from_bits(c.to_bits()))
        .collect()
}

fn subpixel_offset(step: u8) -> f32 {
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

#[cfg(test)]
#[path = "raster_tests.rs"]
mod tests;
