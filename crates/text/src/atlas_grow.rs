//! Growing the coverage page: when the glyphs the current frame needs
//! do not fit, the page doubles, up to its largest side, and every
//! glyph is packed into it again. The text pipeline sees the larger
//! page and makes a texture to match.

use crate::atlas::GlyphAtlas;
use crate::atlas_page::{AtlasPage, PADDING};
use crate::atlas_types::AtlasError;

impl GlyphAtlas {
    /// True when a `width` by `height` glyph fits the largest page.
    pub(crate) fn fits_largest_page(&self, width: u32, height: u32) -> bool {
        width + PADDING <= self.max_page && height + PADDING <= self.max_page
    }

    /// Room for a `width` by `height` glyph in a larger page, or
    /// [`AtlasError::Full`] when the largest page does not hold it with
    /// this frame's glyphs.
    pub(crate) fn grow_until_fits(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<[u32; 2], AtlasError> {
        while self.grow() {
            if self.repack().is_err() {
                continue;
            }
            if let Some(origin) = self.coverage.allocate(width, height) {
                return Ok(origin);
            }
        }
        Err(self.full())
    }

    /// Replace the coverage page with an empty one twice its side, up to
    /// the largest; false when it is the largest already.
    fn grow(&mut self) -> bool {
        let side = self.coverage.size();
        if side >= self.max_page {
            return false;
        }
        let larger = side.saturating_mul(2).min(self.max_page);
        self.coverage = AtlasPage::new(larger, 1);
        true
    }
}

#[cfg(test)]
#[path = "atlas_grow_tests.rs"]
mod tests;
