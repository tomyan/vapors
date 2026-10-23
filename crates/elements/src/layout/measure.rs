//! Measuring an anonymous text box for Taffy: Parley lays the text out
//! at the width Taffy offers, at the display scale so line breaks match
//! what is drawn, and the size comes back in CSS pixels.

use taffy::{AvailableSpace, Size};
use vapors_text::{FontSystem, Layout, LayoutOptions};

use crate::style::TextStyle;

/// The size of `text` in `style`: its width is the widest line, its
/// height the sum of its lines.
pub(crate) fn measure_text(
    fonts: &mut FontSystem,
    scale: f32,
    text: &str,
    style: &TextStyle,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
) -> Size<f32> {
    let width = known.width.or(match available.width {
        AvailableSpace::Definite(width) => Some(width),
        AvailableSpace::MinContent => Some(0.0),
        AvailableSpace::MaxContent => None,
    });
    let layout = text_layout(fonts, scale, text, style, width);
    Size {
        width: known.width.unwrap_or(layout.width() / scale),
        height: known.height.unwrap_or(layout.height() / scale),
    }
}

/// `text` laid out at `width` CSS pixels (`None` never wraps).
pub(crate) fn text_layout(
    fonts: &mut FontSystem,
    scale: f32,
    text: &str,
    style: &TextStyle,
    width: Option<f32>,
) -> Layout {
    let options = LayoutOptions {
        width,
        scale,
        ..LayoutOptions::default()
    };
    Layout::new(fonts, text, &style.for_layout(), &options)
}
