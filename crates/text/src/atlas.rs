//! The glyph atlas: rasterised glyphs packed into pages, found again by
//! font, size, variation, subpixel offset and hinting. Coverage goes to
//! an R8 page, colour glyphs to an RGBA8 page (not rasterised yet; the
//! page exists so the renderer's shape is settled). When a page fills,
//! the least recently used glyphs not in the current frame are evicted
//! until the new one fits (`atlas_evict.rs`), or the page grows
//! (`atlas_grow.rs`). A glyph larger than a page is refused once, logged
//! with its size, and `.notdef` stands in from then on
//! (`atlas_notdef.rs`). Icons share the coverage page (`atlas_icon.rs`).
//! Glyphs that kept bytes name are pinned (`atlas_pins.rs`).

use std::collections::{HashMap, HashSet};

use crate::atlas_page::AtlasPage;
use crate::atlas_pins::AtlasPins;
pub use crate::atlas_types::{AtlasConfig, AtlasError, AtlasGlyph, PageKind};
use crate::atlas_types::{AtlasKey, GlyphKey};
use crate::raster::{Coverage, GlyphRequest, Rasterizer};

/// The colour page's largest side, 4 MiB of RGBA: nothing draws from it
/// until colour glyphs land, which sizes it.
const COLOUR_PAGE: u32 = 1024;

pub(crate) struct Entry {
    pub(crate) glyph: AtlasGlyph,
    pub(crate) coverage: Coverage,
    /// Stamp of the last `get`; higher is more recent.
    pub(crate) last_used: u64,
    /// Frame of the last `get`; the current frame's glyphs are kept.
    pub(crate) frame: u64,
    /// Kept bytes naming this entry (`atlas_pins.rs`): never evicted
    /// while above 0.
    pub(crate) pins: u32,
}

/// Rasterised glyphs packed into pages.
pub struct GlyphAtlas {
    pub(crate) coverage: AtlasPage,
    /// The side the coverage page grows to, and no further.
    pub(crate) max_page: u32,
    colour: AtlasPage,
    pub(crate) rasterizer: Rasterizer,
    pub(crate) entries: HashMap<AtlasKey, Entry>,
    pub(crate) too_large: HashSet<GlyphKey>,
    pub(crate) frame: u64,
    stamp: u64,
    pub(crate) evictions: u64,
    pub(crate) rasterisations: u64,
    /// Bumped by every repack and pin clear: uv's from before it may
    /// name moved or evicted glyphs.
    pub(crate) generation: u64,
    /// Bumped when every pin is cleared: pins taken before are void.
    pub(crate) pin_epoch: u64,
    pub(crate) lookups: u64,
}

impl GlyphAtlas {
    /// Empty pages of `config.page_size`, the colour page at most 1024;
    /// panics if `page_size` is over `max_page_size` (`AtlasConfig::checked`).
    #[must_use]
    pub fn new(config: AtlasConfig) -> Self {
        config.assert_first_page_fits();
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
            generation: 0,
            pin_epoch: 0,
            lookups: 0,
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
        self.get_pinned(request, None)
    }

    /// As [`GlyphAtlas::get`], pinning the entry it answers with into
    /// `pins` when given (`atlas_pins.rs`).
    pub(crate) fn get_pinned(
        &mut self,
        request: &GlyphRequest<'_>,
        pins: Option<&mut AtlasPins>,
    ) -> Result<AtlasGlyph, AtlasError> {
        let glyph_key = GlyphKey::of(request);
        if self.too_large.contains(&glyph_key) {
            return self.notdef_for(request, pins);
        }
        let key = AtlasKey::Glyph(glyph_key);
        let placed = match self.cached(&key) {
            Some(placed) => placed,
            None => match self.rasterized(request)? {
                Ok(coverage) => self.insert(key.clone(), coverage)?,
                Err(coverage) => return self.refuse(request, &coverage, pins),
            },
        };
        self.pin(&key, pins);
        Ok(placed)
    }

    /// The entry under `key`, marked used by this frame; counted as a
    /// lookup.
    pub(crate) fn cached(&mut self, key: &AtlasKey) -> Option<AtlasGlyph> {
        self.lookups += 1;
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
            pins: 0,
        };
        self.entries.insert(key, entry);
        Ok(glyph)
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
