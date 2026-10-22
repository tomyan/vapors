//! What the atlas hands out and how it fails.

use std::fmt;

use crate::raster::{GlyphRequest, Hinting, RasterError};

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
#[derive(Debug, Clone, PartialEq)]
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
    /// `.notdef` itself is larger than a page, so nothing can stand in.
    TooLarge {
        /// Blob id of the font.
        font: u64,
        /// Glyph id.
        glyph: u32,
        /// Font size in pixels.
        size: f32,
        /// Bitmap width and height.
        bitmap: [u32; 2],
        /// Page side in pixels.
        page_size: u32,
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
            Self::TooLarge {
                font,
                glyph,
                size,
                bitmap,
                page_size,
            } => write!(
                f,
                "font {font} glyph {glyph} at {size} px is {}x{} and the atlas page is \
                 {page_size}x{page_size}, and .notdef cannot stand in. Fix: raise \
                 AtlasConfig::page_size or draw this text smaller; large text as paths is later work.",
                bitmap[0], bitmap[1]
            ),
        }
    }
}

impl std::error::Error for AtlasError {}

/// What makes two glyph requests the same atlas entry.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(crate) struct GlyphKey {
    pub(crate) font: u64,
    pub(crate) index: u32,
    pub(crate) id: u32,
    pub(crate) size_bits: u32,
    pub(crate) coords: Vec<i16>,
    pub(crate) subpixel: u8,
    pub(crate) hinting: Hinting,
}

impl GlyphKey {
    pub(crate) fn of(request: &GlyphRequest<'_>) -> Self {
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
