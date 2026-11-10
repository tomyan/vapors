//! The largest an inline `<svg>` icon may be drawn (docs/icons.md, The
//! subset): each of its paths takes a mask the size of its content box
//! in the glyph atlas's 1024 px page, so a large icon fails the frame
//! at draw time. Refused before layout instead, naming the element.

use std::collections::BTreeMap;

use crate::svg::positive;

/// The most device px an svg's width or height may be: 9 masks of
/// this size fill one 1024 px atlas page (each is padded by 1 px;
/// pinned by the test in `svg_limit_tests.rs`), and touche's icons are
/// 16 px, 48 px at 3x.
pub(crate) const MAX_DEVICE_PX: f32 = 256.0;

/// A refusal when the svg's `width` or `height` times `scale` is over
/// [`MAX_DEVICE_PX`]; none when either is missing or unreadable, which
/// `svg_checks` refuses on its own.
pub(crate) fn reason(attributes: &BTreeMap<String, String>, scale: f32) -> Option<String> {
    let [width, height] = ["width", "height"].map(|name| positive(attributes.get(name)?));
    let (width, height) = (width?, height?);
    if width.max(height) * scale <= MAX_DEVICE_PX {
        return None;
    }
    Some(format!(
        "is {width} x {height} CSS px, {} x {} device px at the viewport's scale of {scale}, over the {MAX_DEVICE_PX} px limit an icon is drawn at (docs/icons.md); draw it smaller, or as an image",
        width * scale,
        height * scale
    ))
}

#[cfg(test)]
#[path = "svg_limit_tests.rs"]
mod tests;
