//! Inline-level elements (`display: inline`): their text joins their
//! parent's inline run (`layout::inline`). Their own box edges are not
//! laid out yet, so a margin, padding or border on one is refused
//! rather than dropped.

use taffy::{LengthPercentage, LengthPercentageAuto, Style};
use vapors_css::value::Value;

use super::values::{Reader, keyword};

/// Whether the element is inline-level: `display: inline`, unless its
/// parent is a flex or grid container, which blockifies it.
pub(crate) fn is_inline(reader: &mut Reader<'_>, blockified: bool) -> bool {
    !blockified && reader.read("display", inline_keyword, false)
}

fn inline_keyword(value: &Value) -> Result<bool, String> {
    keyword(value).map(|k| k == "inline")
}

/// Refuses each margin, padding and border side of an inline element
/// that is not zero, naming it.
pub(crate) fn refuse_box_edges(reader: &mut Reader<'_>, style: &Style) {
    let zero = LengthPercentage::length(0.0);
    let auto_zero = LengthPercentageAuto::length(0.0);
    let sides = ["top", "right", "bottom", "left"];
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
    for (index, side) in sides.into_iter().enumerate() {
        if margins[index] != auto_zero {
            reader.refuse(margin_name(side), edge_reason());
        }
        if paddings[index] != zero {
            reader.refuse(padding_name(side), edge_reason());
        }
        if borders[index] != zero {
            reader.refuse(border_name(side), edge_reason());
        }
    }
}

fn edge_reason() -> String {
    "is not laid out on an inline element (display: inline) yet; not scheduled in docs/F2c.md. Use display: inline-block in a flex container, or move it to the block around the text".to_owned()
}

fn margin_name(side: &str) -> &'static str {
    match side {
        "top" => "margin-top",
        "right" => "margin-right",
        "bottom" => "margin-bottom",
        _ => "margin-left",
    }
}

fn padding_name(side: &str) -> &'static str {
    match side {
        "top" => "padding-top",
        "right" => "padding-right",
        "bottom" => "padding-bottom",
        _ => "padding-left",
    }
}

fn border_name(side: &str) -> &'static str {
    match side {
        "top" => "border-top-width",
        "right" => "border-right-width",
        "bottom" => "border-bottom-width",
        _ => "border-left-width",
    }
}
