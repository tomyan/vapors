//! The style a `gpu-surface` refuses (docs/gpu-surface.md): padding, so
//! that a pointer event's offset on the surface is surface-local; a
//! border-radius (no rounded clip); and `display: inline`
//! (inline-block content is not laid out).

use taffy::{LengthPercentage, Style};
use vapors_css::property::longhands::DISPLAY;

use super::decoration::Decoration;
use super::sides::{BORDER_RADII, PADDINGS};
use super::values::Reader;

/// Refuses, by property, what a `gpu-surface` styled `layout`,
/// `decoration` and (when `inline`) `display: inline` cannot draw.
pub(crate) fn refuse(
    reader: &mut Reader<'_>,
    layout: &Style,
    decoration: &Decoration,
    inline: bool,
) {
    if inline {
        reader.refuse(DISPLAY, "is inline on a gpu-surface, and inline-block content is not laid out yet; give it display: block, or put it in a flex or grid container".to_owned());
    }
    for (property, radius) in BORDER_RADII.into_iter().zip(decoration.radii) {
        if radius > 0.0 {
            reader.refuse(property, "rounds a gpu-surface, and a surface is not clipped to rounded corners (docs/gpu-surface.md); drop the radius".to_owned());
        }
    }
    for (property, side) in PADDINGS.into_iter().zip(paddings(layout)) {
        if !is_zero(side) {
            reader.refuse(property, "pads a gpu-surface, which would make pointer offsets on it differ from surface-local coordinates (docs/gpu-surface.md); put the padding on a wrapping element".to_owned());
        }
    }
}

fn paddings(layout: &Style) -> [LengthPercentage; 4] {
    let padding = layout.padding;
    [padding.top, padding.right, padding.bottom, padding.left]
}

/// A zero length or a zero percentage: no padding.
fn is_zero(side: LengthPercentage) -> bool {
    side == LengthPercentage::length(0.0) || side == LengthPercentage::percent(0.0)
}
