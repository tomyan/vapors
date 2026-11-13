//! Glyphs larger than the largest page: each is refused once, logged
//! with its size, and `.notdef` stands in for it from then on.

use crate::atlas::GlyphAtlas;
use crate::atlas_pins::AtlasPins;
use crate::atlas_types::{AtlasError, AtlasGlyph, GlyphKey};
use crate::raster::{Coverage, GlyphRequest};

impl GlyphAtlas {
    /// Glyphs refused for being larger than a page.
    #[must_use]
    pub fn too_large(&self) -> usize {
        self.too_large.len()
    }

    /// `request`'s coverage, counted; `Err` holding it when it is larger
    /// than the largest page.
    pub(crate) fn rasterized(
        &mut self,
        request: &GlyphRequest<'_>,
    ) -> Result<Result<Coverage, Coverage>, AtlasError> {
        let coverage = self
            .rasterizer
            .rasterize(request)
            .map_err(AtlasError::Raster)?;
        self.rasterisations += 1;
        if self.fits_largest_page(coverage.width, coverage.height) {
            Ok(Ok(coverage))
        } else {
            Ok(Err(coverage))
        }
    }

    /// Log the oversized glyph once, remember it, and answer with
    /// `.notdef`, pinned into `pins` when given.
    pub(crate) fn refuse(
        &mut self,
        request: &GlyphRequest<'_>,
        coverage: &Coverage,
        pins: Option<&mut AtlasPins>,
    ) -> Result<AtlasGlyph, AtlasError> {
        let error = AtlasError::TooLarge {
            font: request.font.data.id(),
            glyph: request.id,
            size: request.size,
            bitmap: [coverage.width, coverage.height],
            page_size: self.max_page,
        };
        if request.id == 0 {
            return Err(error);
        }
        log::error!("{error}; drawing .notdef");
        self.too_large.insert(GlyphKey::of(request));
        self.notdef_for(request, pins)
    }

    /// `.notdef` in `request`'s font and size, pinned into `pins` when
    /// given.
    pub(crate) fn notdef_for(
        &mut self,
        request: &GlyphRequest<'_>,
        pins: Option<&mut AtlasPins>,
    ) -> Result<AtlasGlyph, AtlasError> {
        let notdef = GlyphRequest { id: 0, ..*request };
        self.get_pinned(&notdef, pins).map(|glyph| AtlasGlyph {
            notdef: true,
            ..glyph
        })
    }
}
