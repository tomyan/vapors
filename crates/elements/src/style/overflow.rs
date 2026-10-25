//! `overflow-x` and `overflow-y` (F2c S8, S9): `visible`; `hidden` and
//! `clip`, which cut the element's descendants off at its padding box;
//! and `scroll` and `auto`, which do that and make it a scroll
//! container (`crate::scroll`). Axes that differ are refused: CSS turns
//! `hidden` beside `visible` into a scroll container on one axis, and a
//! one-axis `clip` is not built. A clip with rounded corners is refused
//! until a slice needs it. The root's overflow, and the body's when the
//! root's is `visible`, belong to the viewport (CSS Overflow 3 section
//! 3.3), which vapors does not scroll: there `scroll` and `auto` are
//! refused.

use taffy::{Overflow, Point, Style};
use vapors_css::value::Value;

use super::decoration::Decoration;
use super::values::{Reader, keyword, unknown};

/// Reads both axes into `style`.
pub(crate) fn overflow(reader: &mut Reader<'_>, style: &mut Style) {
    let x = reader.read("overflow-x", scheme, Overflow::Visible);
    let y = reader.read("overflow-y", scheme, Overflow::Visible);
    if x != y {
        reader.refuse(
            "overflow-y",
            format!("is {y:?} where overflow-x is {x:?}: axes that differ make a scroll container (F2c S9) or a one-axis clip, which vapors does not build; give both axes one value"),
        );
    }
    style.overflow = Point { x, y };
}

/// Refuses a clip with rounded corners.
pub(crate) fn refuse_rounded_clip(reader: &mut Reader<'_>, style: &Style, decoration: &Decoration) {
    if style.overflow.x != Overflow::Visible && decoration.radii.iter().any(|r| *r > 0.0) {
        reader.refuse(
            "overflow-x",
            "clips to a box with a border-radius, and rounded clips are refused until a slice needs them (docs/F2c.md); remove the radius or the clip".to_owned(),
        );
    }
}

/// Whether an element's overflow belongs to the viewport: the root's
/// always, the body's when the root's is `visible`.
pub(crate) fn to_viewport(is_root: bool, is_body: bool, root_visible: bool) -> bool {
    is_root || (is_body && root_visible)
}

/// Refuses `scroll` and `auto` where they would scroll the viewport.
pub(crate) fn refuse_viewport_scroll(reader: &mut Reader<'_>, style: &Style, to_viewport: bool) {
    if to_viewport && style.overflow.x == Overflow::Scroll {
        reader.refuse(
            "overflow-x",
            "scrolls the viewport (html's overflow, or body's while html's is visible), which vapors does not do; give the scrolling content its own element with a height, or set html to overflow: hidden".to_owned(),
        );
    }
}

fn scheme(value: &Value) -> Result<Overflow, String> {
    match keyword(value)? {
        "visible" => Ok(Overflow::Visible),
        "hidden" => Ok(Overflow::Hidden),
        "clip" => Ok(Overflow::Clip),
        "scroll" | "auto" => Ok(Overflow::Scroll),
        other => Err(unknown(other)),
    }
}
