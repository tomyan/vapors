//! What the painter refuses: a target format its shaders do not write
//! correctly, and, drawing over, a document whose canvas has a colour.

use std::fmt;

use vapors_elements::CanvasFrom;
use vapors_text::AtlasError;

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

/// Why [`ScenePainter::draw_over`](super::ScenePainter::draw_over) drew
/// nothing or an incomplete frame.
#[derive(Debug)]
pub enum DrawOverError {
    /// `html` or `body` sets a background: it is the canvas's, which
    /// drawing over never paints.
    CanvasBackground(CanvasFrom),
    /// A glyph could not be drawn, as for `draw`.
    Atlas(AtlasError),
}

impl fmt::Display for DrawOverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanvasBackground(from) => write!(
                f,
                "the document's {} sets a background, which becomes the canvas's, and drawing over paints no canvas; set the background on an element inside it",
                from.tag()
            ),
            Self::Atlas(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DrawOverError {}
