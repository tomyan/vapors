//! One glyph to coverage on the CPU: skrifa's outline and hinting, then
//! zeno's rasteriser, at one of [`SUBPIXEL_STEPS`] horizontal offsets.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use parlance::NormalizedCoord;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, HintingInstance, OutlineGlyphCollection};
use skrifa::{GlyphId, MetadataProvider};

use crate::pen::PathCommands;
use crate::raster_types::subpixel_offset;
pub use crate::raster_types::{
    Coverage, GlyphRequest, Hinting, RasterError, SUBPIXEL_STEPS, subpixel_of,
};

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
        Ok(path.rasterize(subpixel_offset(request.subpixel), request.skew))
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

#[cfg(test)]
#[path = "raster_tests.rs"]
mod tests;
