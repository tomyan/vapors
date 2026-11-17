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
    /// The format is not guaranteed to draw at the sample count asked
    /// for (`flat-scene-f7.md`, L2 of its F7a1 review).
    Samples(vapors_renderer::PassFormat),
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
            Self::Samples(pass) => write!(
                f,
                "ScenePainter at {} samples on {:?}: the format is not guaranteed to draw at that count on this device; make the painter at 1 sample or on {}",
                pass.samples.count(),
                pass.format,
                supported_names()
            ),
        }
    }
}

impl std::error::Error for NewPainterError {}

/// The formats a painter draws to ([`SUPPORTED`]), named for a fix.
fn supported_names() -> String {
    let names: Vec<String> = SUPPORTED
        .iter()
        .map(|format| format!("{format:?}"))
        .collect();
    names.join(" or ")
}

/// `pass`, when its format is guaranteed to draw at its sample count on
/// a device with `features` (wgpu's guaranteed format features, which
/// need no adapter); else the refusal.
///
/// # Errors
///
/// [`NewPainterError::Samples`] as said.
pub(super) fn supported_samples(
    pass: vapors_renderer::PassFormat,
    features: wgpu::Features,
) -> Result<vapors_renderer::PassFormat, NewPainterError> {
    let flags = pass.format.guaranteed_format_features(features).flags;
    if flags.sample_count_supported(pass.samples.count()) {
        Ok(pass)
    } else {
        Err(NewPainterError::Samples(pass))
    }
}

/// Why [`ScenePainter::draw_over`](super::ScenePainter::draw_over) drew
/// nothing or an incomplete frame.
#[derive(Debug)]
pub enum DrawOverError {
    /// `html` or `body` sets a background: it is the canvas's, which
    /// drawing over never paints.
    CanvasBackground(CanvasFrom),
    /// A 4-sample painter draws into its own 4x texture, which holds
    /// nothing of what the target holds (`docs/plans/flat-scene-f7.md`,
    /// Q4).
    Multisampled,
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
            Self::Multisampled => write!(
                f,
                "draw_over at 4 samples: the painter draws into its own 4x texture, which holds nothing of what the target holds, so there is nothing to draw over; draw the page with draw, or make the painter at 1 sample"
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

#[cfg(test)]
#[path = "painter_errors_tests.rs"]
mod tests;
