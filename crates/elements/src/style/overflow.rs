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
use vapors_css::property::longhands::{OVERFLOW_X, OVERFLOW_Y};
use vapors_css::value::Value;

use super::decoration::Decoration;
use super::values::{Reader, keyword, unknown};

/// Reads both axes into `style`. The cascade has computed them as a
/// pair (`vapors_css::cascade::overflow`), so what is left that differs
/// is two scrolling axes, which lay out and scroll per axis, and
/// `clip` beside `visible`, which is refused.
pub(crate) fn overflow(reader: &mut Reader<'_>, style: &mut Style) {
    let x = reader.read(OVERFLOW_X, scheme, Overflow::Visible);
    let y = reader.read(OVERFLOW_Y, scheme, Overflow::Visible);
    let pair = match (x, y) {
        (Overflow::Clip, Overflow::Visible) => Some(("clip", "visible")),
        (Overflow::Visible, Overflow::Clip) => Some(("visible", "clip")),
        _ => None,
    };
    if let Some((x, y)) = pair {
        reader.refuse(
            OVERFLOW_Y,
            format!("overflow-x: {x}; overflow-y: {y} clips one axis and leaves the other visible, which vapors cannot draw yet (its clip is a padding box, F2c S8); give both axes clip or hidden, or make the visible axis hidden (reproduce: cargo test -p vapors-elements --test tests scroll_axes)"),
        );
    }
    style.overflow = Point { x, y };
}

/// Refuses a clip with rounded corners.
pub(crate) fn refuse_rounded_clip(reader: &mut Reader<'_>, style: &Style, decoration: &Decoration) {
    if clips(style) && decoration.radii.iter().any(|r| *r > 0.0) {
        reader.refuse(
            OVERFLOW_X,
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
    if to_viewport && scrolls(style) {
        reader.refuse(
            OVERFLOW_X,
            "scrolls the viewport (html's overflow, or body's while html's is visible), which vapors does not do; give the scrolling content its own element with a height, or set html to overflow: hidden".to_owned(),
        );
    }
}

/// Refuses a text or number field that scrolls: its text origin and
/// marks come from its text child's rect, which is in the field's own
/// scroll frame, while they are kept in the frame the field is in
/// (`frames.rs`, IU4).
pub(crate) fn refuse_scrolling_field(reader: &mut Reader<'_>, style: &Style, field: bool) {
    if field && scrolls(style) {
        reader.refuse(
            OVERFLOW_X,
            "scrolls a text or number field, which vapors does not do: its text is placed from the field's box; use overflow: hidden or clip".to_owned(),
        );
    }
}

/// Whether either axis of `style` scrolls (`auto` or `scroll`).
pub(crate) fn scrolls(style: &Style) -> bool {
    style.overflow.x == Overflow::Scroll || style.overflow.y == Overflow::Scroll
}

/// Whether either axis of `style` cuts its content off.
pub(crate) fn clips(style: &Style) -> bool {
    style.overflow.x != Overflow::Visible || style.overflow.y != Overflow::Visible
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
