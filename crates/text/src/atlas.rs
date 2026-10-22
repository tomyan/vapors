//! The glyph atlas: rasterised glyphs packed into pages, found again by
//! font, size, variation, subpixel offset and hinting. Coverage goes to
//! an R8 page, colour glyphs to an RGBA8 page (not rasterised yet; the
//! page exists so the renderer's shape is settled). When a page fills,
//! every glyph the current frame has not used is evicted and the rest
//! repacked; both evictions and rasterisations are counted for tests.

use std::collections::HashMap;
use std::fmt;

use crate::atlas_page::AtlasPage;
use crate::raster::{Coverage, GlyphRequest, Hinting, RasterError, Rasterizer};

/// Atlas sizing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasConfig {
    /// Side of each square page in pixels.
    pub page_size: u32,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self { page_size: 1024 }
    }
}

/// Which page a glyph is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageKind {
    /// 8-bit coverage.
    Coverage,
    /// 8-bit RGBA colour.
    Colour,
}

/// Where a glyph is in the atlas and how to place it on the baseline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasGlyph {
    /// The page.
    pub page: PageKind,
    /// `[x, y, width, height]` in page pixels.
    pub rect: [u32; 4],
    /// Left edge from the glyph origin, pixels.
    pub left: i32,
    /// Top edge from the baseline, pixels; negative is up.
    pub top: i32,
    /// True when this is `.notdef` standing in for a glyph that failed.
    pub notdef: bool,
}

impl AtlasGlyph {
    /// True for glyphs with no ink.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rect[2] == 0 || self.rect[3] == 0
    }
}

/// Why a glyph could not be placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtlasError {
    /// Rasterisation failed outright.
    Raster(RasterError),
    /// The glyphs this frame needs do not fit one page.
    Full {
        /// Page side in pixels.
        page_size: u32,
        /// Glyphs used this frame when it overflowed.
        used_this_frame: usize,
    },
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raster(error) => error.fmt(f),
            Self::Full {
                page_size,
                used_this_frame,
            } => write!(
                f,
                "glyph atlas page of {page_size}x{page_size} is full with {used_this_frame} glyphs \
                 in use this frame. Fix: raise AtlasConfig::page_size or draw less text per frame."
            ),
        }
    }
}

impl std::error::Error for AtlasError {}

#[derive(Clone, Hash, PartialEq, Eq)]
struct GlyphKey {
    font: u64,
    index: u32,
    id: u32,
    size_bits: u32,
    coords: Vec<i16>,
    subpixel: u8,
    hinting: Hinting,
}

impl GlyphKey {
    fn of(request: &GlyphRequest<'_>) -> Self {
        Self {
            font: request.font.data.id(),
            index: request.font.index,
            id: request.id,
            size_bits: request.size.to_bits(),
            coords: request.coords.iter().map(|c| c.to_bits()).collect(),
            subpixel: request.subpixel,
            hinting: request.hinting,
        }
    }
}

struct Entry {
    glyph: AtlasGlyph,
    coverage: Coverage,
    last_used: u64,
}

/// Rasterised glyphs packed into pages.
pub struct GlyphAtlas {
    coverage: AtlasPage,
    colour: AtlasPage,
    rasterizer: Rasterizer,
    entries: HashMap<GlyphKey, Entry>,
    frame: u64,
    evictions: u64,
    rasterisations: u64,
}

impl GlyphAtlas {
    /// Empty pages of `config.page_size`.
    #[must_use]
    pub fn new(config: AtlasConfig) -> Self {
        Self {
            coverage: AtlasPage::new(config.page_size, 1),
            colour: AtlasPage::new(config.page_size, 4),
            rasterizer: Rasterizer::new(),
            entries: HashMap::new(),
            frame: 1,
            evictions: 0,
            rasterisations: 0,
        }
    }

    /// Start a frame: glyphs fetched from now on are the ones eviction keeps.
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
    /// if needed.
    ///
    /// # Errors
    ///
    /// [`AtlasError`] when it cannot be rasterised or the frame's glyphs
    /// overflow a page.
    pub fn get(&mut self, request: &GlyphRequest<'_>) -> Result<AtlasGlyph, AtlasError> {
        let key = GlyphKey::of(request);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.frame;
            return Ok(entry.glyph);
        }
        let coverage = self
            .rasterizer
            .rasterize(request)
            .map_err(AtlasError::Raster)?;
        self.rasterisations += 1;
        let glyph = self.place(&coverage)?;
        self.entries.insert(
            key,
            Entry {
                glyph,
                coverage,
                last_used: self.frame,
            },
        );
        Ok(glyph)
    }

    fn place(&mut self, coverage: &Coverage) -> Result<AtlasGlyph, AtlasError> {
        if coverage.is_empty() {
            return Ok(glyph_at([0, 0], coverage));
        }
        let (w, h) = (coverage.width, coverage.height);
        let origin = if let Some(origin) = self.coverage.allocate(w, h) {
            origin
        } else {
            self.evict_unused();
            self.coverage.allocate(w, h).ok_or(AtlasError::Full {
                page_size: self.coverage.size(),
                used_this_frame: self.entries.len(),
            })?
        };
        self.coverage
            .blit(origin, coverage.width, coverage.height, &coverage.data);
        Ok(glyph_at(origin, coverage))
    }

    /// Drop every glyph this frame has not used and repack the rest.
    fn evict_unused(&mut self) {
        let frame = self.frame;
        let before = self.entries.len();
        self.entries.retain(|_, entry| entry.last_used == frame);
        self.evictions += (before - self.entries.len()) as u64;
        self.coverage.clear();
        for entry in self.entries.values_mut() {
            if entry.coverage.is_empty() {
                continue;
            }
            let (w, h) = (entry.coverage.width, entry.coverage.height);
            let origin = self
                .coverage
                .allocate(w, h)
                .expect("survivors fitted before, so they fit a cleared page");
            self.coverage.blit(origin, w, h, &entry.coverage.data);
            entry.glyph = glyph_at(origin, &entry.coverage);
        }
    }
}

fn glyph_at(origin: [u32; 2], coverage: &Coverage) -> AtlasGlyph {
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
