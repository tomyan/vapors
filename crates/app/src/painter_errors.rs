//! What the painter refuses: a target format its shaders do not write
//! correctly, drawing over, a document whose canvas has a colour, and a
//! frame it cannot draw completely.

use std::fmt;

use vapors_elements::CanvasFrom;
use vapors_renderer::ArenaFull;
use vapors_text::{AtlasError, AtlasSizeError};

use crate::shape_store::ShapeError;
use crate::surface::SurfaceError;

/// The target formats [`ScenePainter`](super::ScenePainter) draws: 8-bit,
/// not sRGB, as the platform's surfaces are.
pub const SUPPORTED: [wgpu::TextureFormat; 2] = [
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Bgra8Unorm,
];

/// A target format the painter would draw wrong colours into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedFormat(pub wgpu::TextureFormat);

impl fmt::Display for UnsupportedFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ScenePainter cannot draw into {:?}: its shaders write sRGB-encoded values, so only Rgba8Unorm and Bgra8Unorm show the right colours; draw into one of those",
            self.0
        )
    }
}

impl std::error::Error for UnsupportedFormat {}

/// Why a [`ScenePainter`](super::ScenePainter) could not be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewPainterError {
    /// The target format would draw wrong colours.
    Format(UnsupportedFormat),
    /// The device cannot hold the glyph atlas's pages.
    Atlas(AtlasSizeError),
}

impl From<UnsupportedFormat> for NewPainterError {
    fn from(error: UnsupportedFormat) -> Self {
        Self::Format(error)
    }
}

impl From<AtlasSizeError> for NewPainterError {
    fn from(error: AtlasSizeError) -> Self {
        Self::Atlas(error)
    }
}

impl fmt::Display for NewPainterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(error) => error.fmt(f),
            Self::Atlas(error) => write!(f, "ScenePainter's glyph atlas: {error}"),
        }
    }
}

impl std::error::Error for NewPainterError {}

/// Why [`ScenePainter::draw_over`](super::ScenePainter::draw_over) drew
/// nothing or an incomplete frame.
#[derive(Debug)]
pub enum DrawOverError {
    /// `html` or `body` sets a background: it is the canvas's, which
    /// drawing over never paints.
    CanvasBackground(CanvasFrom),
    /// As for `draw`.
    Paint(PaintError),
}

impl fmt::Display for DrawOverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanvasBackground(from) => write!(
                f,
                "the document's {} sets a background, which becomes the canvas's, and drawing over paints no canvas; set the background on an element inside it",
                from.tag()
            ),
            Self::Paint(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DrawOverError {}

/// Why [`ScenePainter::draw`](super::ScenePainter::draw) drew an
/// incomplete frame.
#[derive(Debug)]
pub enum PaintError {
    /// A glyph could not be rasterised, or a frame's glyphs do not fit
    /// the atlas page.
    Atlas(AtlasError),
    /// A draw's bytes need a buffer over the device's `max_buffer_size`.
    Arena(ArenaFull),
    /// A `gpu-surface`, by name, with no renderer mounted.
    Unmounted(String),
    /// Two `gpu-surface`s in one frame share this name.
    DuplicateName(String),
    /// A surface's backing would be larger than the device's largest 2D
    /// texture.
    TooLarge {
        /// The surface's name.
        surface: String,
        /// The backing's width in device pixels.
        width: u32,
        /// The backing's height in device pixels.
        height: u32,
        /// The device's `max_texture_dimension_2d`.
        limit: u32,
    },
    /// A shape item names an entry the painter's store cannot draw.
    Shape(ShapeError),
    /// A surface's renderer failed in `mount`, `resize` or `render`, or
    /// made a GPU validation error there.
    Renderer {
        /// The surface's name.
        surface: String,
        /// What the renderer reported, or the GPU's validation error.
        error: SurfaceError,
    },
}

impl From<AtlasError> for PaintError {
    fn from(error: AtlasError) -> Self {
        Self::Atlas(error)
    }
}

impl From<ArenaFull> for PaintError {
    fn from(error: ArenaFull) -> Self {
        Self::Arena(error)
    }
}

impl fmt::Display for PaintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        surface_errors::describe(self, f)
    }
}

impl std::error::Error for PaintError {}

#[path = "painter_surface_errors.rs"]
mod surface_errors;
pub use surface_errors::MountError;
pub(crate) use surface_errors::Refusal;
