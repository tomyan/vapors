//! Icons in the atlas: rasterised once per paths, stroke widths, view
//! box and device size, packed into the coverage page beside glyphs and
//! evicted with them (docs/icons.md, Paint).

use std::sync::Arc;

use crate::atlas::{AtlasError, AtlasGlyph, GlyphAtlas};
use crate::atlas_pins::AtlasPins;
use crate::atlas_types::AtlasKey;
use crate::icon::Icon;

impl GlyphAtlas {
    /// Where stroke `stroke` of `icon` at `size` device pixels is in the
    /// atlas, rasterising and packing it first if needed. Its `left` and
    /// `top` are 0: the bitmap's corner is the box's. One entry per path,
    /// so each path is its own quad and paint, as in Chromium.
    ///
    /// # Panics
    ///
    /// If `stroke` is not below [`Icon::strokes`]; a caller bug.
    ///
    /// # Errors
    ///
    /// [`AtlasError::IconTooLarge`] when `size` does not fit a page, and
    /// [`AtlasError::Full`] when the frame's glyphs and icons overflow one.
    pub fn icon(
        &mut self,
        icon: &Icon,
        stroke: usize,
        size: [u32; 2],
    ) -> Result<AtlasGlyph, AtlasError> {
        self.icon_pinned((icon, stroke, size), None)
    }

    /// As [`GlyphAtlas::icon`], pinning the entry into `pins` when given
    /// (`atlas_pins.rs`).
    pub(crate) fn icon_pinned(
        &mut self,
        (icon, stroke, size): (&Icon, usize, [u32; 2]),
        pins: Option<&mut AtlasPins>,
    ) -> Result<AtlasGlyph, AtlasError> {
        let key = AtlasKey::Icon {
            bits: Arc::clone(icon.bits(stroke)),
            size,
        };
        let placed = match self.cached(&key) {
            Some(placed) => placed,
            None => self.icon_inserted(&key, (icon, stroke, size))?,
        };
        self.pin(&key, pins);
        Ok(placed)
    }

    /// Rasterises stroke `stroke` of `icon` at `size` and packs it under
    /// `key`.
    fn icon_inserted(
        &mut self,
        key: &AtlasKey,
        (icon, stroke, size): (&Icon, usize, [u32; 2]),
    ) -> Result<AtlasGlyph, AtlasError> {
        if !self.fits_largest_page(size[0], size[1]) {
            let page_size = self.max_page;
            return Err(AtlasError::IconTooLarge { size, page_size });
        }
        self.rasterisations += 1;
        self.insert(key.clone(), icon.rasterize_stroke(stroke, size))
    }
}

#[cfg(test)]
#[path = "atlas_icon_tests.rs"]
mod tests;
