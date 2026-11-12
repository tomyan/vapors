//! The glyph atlas: rasterised glyphs packed into pages, found again by
//! font, size, variation, subpixel offset and hinting. Coverage goes to
//! an R8 page, colour glyphs to an RGBA8 page (not rasterised yet; the
//! page exists so the renderer's shape is settled). When a page fills,
//! the least recently used glyphs not needed by the current frame are
//! evicted until the new one fits (`atlas_evict.rs`). A glyph larger
//! than a page is refused once, logged with its size, and `.notdef`
//! stands in from then on. Evictions, rasterisations and refusals are
//! counted for tests. Icons share the coverage page (`atlas_icon.rs`).
//! When one frame's glyphs do not fit, the coverage page doubles up to
//! its largest side (`atlas_grow.rs`).

use std::collections::{HashMap, HashSet};

use crate::atlas_page::AtlasPage;
pub use crate::atlas_types::{AtlasConfig, AtlasError, AtlasGlyph, PageKind};
use crate::atlas_types::{AtlasKey, GlyphKey};
use crate::raster::{Coverage, GlyphRequest, Rasterizer};

/// The colour page's largest side, 4 MiB of RGBA: nothing draws from it
/// until colour glyphs are rasterised, and that pull request sizes it.
const COLOUR_PAGE: u32 = 1024;

pub(crate) struct Entry {
    pub(crate) glyph: AtlasGlyph,
    pub(crate) coverage: Coverage,
    /// Stamp of the last `get`; higher is more recent.
    pub(crate) last_used: u64,
    /// Frame of the last `get`; the current frame's glyphs are kept.
    pub(crate) frame: u64,
}

/// Rasterised glyphs packed into pages.
pub struct GlyphAtlas {
    pub(crate) coverage: AtlasPage,
    /// The side the coverage page grows to, and no further.
    pub(crate) max_page: u32,
    colour: AtlasPage,
    rasterizer: Rasterizer,
    pub(crate) entries: HashMap<AtlasKey, Entry>,
    too_large: HashSet<GlyphKey>,
    pub(crate) frame: u64,
    stamp: u64,
    pub(crate) evictions: u64,
    pub(crate) rasterisations: u64,
}

impl GlyphAtlas {
    /// Empty pages of `config.page_size`, the colour page at most 1024.
    #[must_use]
    pub fn new(config: AtlasConfig) -> Self {
        Self {
            coverage: AtlasPage::new(config.page_size, 1),
            max_page: config.max_page_size,
            colour: AtlasPage::new(config.page_size.min(COLOUR_PAGE), 4),
            rasterizer: Rasterizer::new(),
            entries: HashMap::new(),
            too_large: HashSet::new(),
            frame: 1,
            stamp: 0,
            evictions: 0,
            rasterisations: 0,
        }
    }

    /// Start a frame: glyphs fetched from now on are never evicted to
    /// make room for others in the same frame.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Glyphs evicted so far.
    #[must_use]
    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    /// Glyphs rasterised so far, re-rasterisations after eviction included.
    #[must_use]
    pub fn rasterisations(&self) -> u64 {
        self.rasterisations
    }

    /// Glyphs refused for being larger than a page.
    #[must_use]
    pub fn too_large(&self) -> usize {
        self.too_large.len()
    }

    /// The R8 page.
    #[must_use]
    pub fn coverage_page(&self) -> &AtlasPage {
        &self.coverage
    }

    /// The R8 page, to mark clean after uploading.
    pub fn coverage_page_mut(&mut self) -> &mut AtlasPage {
        &mut self.coverage
    }

    /// The RGBA8 page.
    #[must_use]
    pub fn colour_page(&self) -> &AtlasPage {
        &self.colour
    }

    /// Where `request` is in the atlas, rasterising and packing it first
    /// if needed. A glyph larger than the page is logged once and comes
    /// back as `.notdef` with `notdef` set.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when it cannot be rasterised, the frame's glyphs
    /// overflow a page, or `.notdef` itself is larger than a page.
    pub fn get(&mut self, request: &GlyphRequest<'_>) -> Result<AtlasGlyph, AtlasError> {
        let glyph_key = GlyphKey::of(request);
        if self.too_large.contains(&glyph_key) {
            return self.notdef_for(request);
        }
        let key = AtlasKey::Glyph(glyph_key);
        if let Some(placed) = self.cached(&key) {
            return Ok(placed);
        }
        let coverage = self
            .rasterizer
            .rasterize(request)
            .map_err(AtlasError::Raster)?;
        self.rasterisations += 1;
        if !self.fits_largest_page(coverage.width, coverage.height) {
            return self.refuse(request, &coverage);
        }
        self.insert(key, coverage)
    }

    /// The entry under `key`, marked used by this frame.
    pub(crate) fn cached(&mut self, key: &AtlasKey) -> Option<AtlasGlyph> {
        self.stamp += 1;
        let entry = self.entries.get_mut(key)?;
        entry.last_used = self.stamp;
        entry.frame = self.frame;
        Some(entry.glyph)
    }

    /// Pack `coverage` and remember it under `key`.
    pub(crate) fn insert(
        &mut self,
        key: AtlasKey,
        coverage: Coverage,
    ) -> Result<AtlasGlyph, AtlasError> {
        let glyph = self.place(&coverage)?;
        let entry = Entry {
            glyph,
            coverage,
            last_used: self.stamp,
            frame: self.frame,
        };
        self.entries.insert(key, entry);
        Ok(glyph)
    }

    /// Log the oversized glyph once, remember it, and answer with `.notdef`.
    fn refuse(
        &mut self,
        request: &GlyphRequest<'_>,
        coverage: &Coverage,
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
        self.notdef_for(request)
    }

    fn notdef_for(&mut self, request: &GlyphRequest<'_>) -> Result<AtlasGlyph, AtlasError> {
        let notdef = GlyphRequest { id: 0, ..*request };
        self.get(&notdef).map(|glyph| AtlasGlyph {
            notdef: true,
            ..glyph
        })
    }

    fn place(&mut self, coverage: &Coverage) -> Result<AtlasGlyph, AtlasError> {
        if coverage.is_empty() {
            return Ok(glyph_at([0, 0], coverage));
        }
        let (w, h) = (coverage.width, coverage.height);
        let origin = match self.coverage.allocate(w, h) {
            Some(origin) => origin,
            None => self.make_room(w, h)?,
        };
        self.coverage.blit(origin, w, h, &coverage.data);
        Ok(glyph_at(origin, coverage))
    }
}

pub(crate) fn glyph_at(origin: [u32; 2], coverage: &Coverage) -> AtlasGlyph {
    AtlasGlyph {
        page: PageKind::Coverage,
        rect: [origin[0], origin[1], coverage.width, coverage.height],
        left: coverage.left,
        top: coverage.top,
        notdef: coverage.notdef,
    }
}

#[cfg(test)]
#[path = "atlas_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "atlas_limits_tests.rs"]
mod limits_tests;
