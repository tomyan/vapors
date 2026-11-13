//! Inline-level elements (`display: inline`): their text joins their
//! parent's inline run (`layout::inline`). Their own box edges are not
//! laid out yet, so a margin, padding or border on one is refused
//! rather than dropped.

use taffy::{LengthPercentage, LengthPercentageAuto, Style};
use vapors_css::property::longhands::{DISPLAY, OVERFLOW_X, POSITION};
use vapors_css::value::Value;

use super::sides::{BORDER_WIDTHS, MARGINS, PADDINGS};
use super::values::{Reader, keyword};

/// Whether the element is inline-level: `display: inline`, unless its
/// parent is a flex or grid container, which blockifies it.
pub(crate) fn is_inline(reader: &mut Reader<'_>, blockified: bool) -> bool {
    !blockified && reader.read(DISPLAY, inline_keyword, false)
}

fn inline_keyword(value: &Value) -> Result<bool, String> {
    keyword(value).map(|k| k == "inline")
}

/// Why a block-level element inside an inline one is refused.
pub(crate) const BLOCK_IN_INLINE: &str = "is block-level inside an inline element, which splits the inline element around it (CSS 2 section 9.2.1.1); not laid out by vapors-elements yet, not scheduled in docs/F2c.md. Make the outer element a block, or this one inline";

/// Refuses each margin, padding and border side of an inline element
/// that is not zero, naming it.
pub(crate) fn refuse_box_edges(reader: &mut Reader<'_>, style: &Style) {
    if style.position != taffy::Position::Static {
        reader.refuse(
            POSITION,
            "is not static on an inline element, which has no box of its own to place yet (a slice after F2c, docs/F2c.md); make the element inline-block in a flex container, or a block".to_owned(),
        );
    }
    if style.overflow.x != taffy::Overflow::Visible {
        reader.refuse(
            OVERFLOW_X,
            "clips an inline element, which has no box of its own to clip to; make it inline-block in a flex container, or a block".to_owned(),
        );
    }
    let zero = LengthPercentage::length(0.0);
    let auto_zero = LengthPercentageAuto::length(0.0);
    let margins = [
        style.margin.top,
        style.margin.right,
        style.margin.bottom,
        style.margin.left,
    ];
    let paddings = [
        style.padding.top,
        style.padding.right,
        style.padding.bottom,
        style.padding.left,
    ];
    let borders = [
        style.border.top,
        style.border.right,
        style.border.bottom,
        style.border.left,
    ];
    for index in 0..4 {
        if margins[index] != auto_zero {
            reader.refuse(MARGINS[index], edge_reason());
        }
        if paddings[index] != zero {
            reader.refuse(PADDINGS[index], edge_reason());
        }
        if borders[index] != zero {
            reader.refuse(BORDER_WIDTHS[index], edge_reason());
        }
    }
}

fn edge_reason() -> String {
    "is not laid out on an inline element (display: inline) yet; not scheduled in docs/F2c.md. Use display: inline-block in a flex container, or move it to the block around the text".to_owned()
}
