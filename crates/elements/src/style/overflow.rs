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

use taffy::{CompactLength, LengthPercentage, Overflow, Point, Style};
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

/// Refuses a clip with rounded corners, except on a text field whose
/// text scrolls on x, so is clipped to its content box on x
/// (`clip.rs`), clear of every corner's curve, where the rectangle is
/// the rounded clip.
pub(crate) fn refuse_rounded_clip(
    reader: &mut Reader<'_>,
    style: &Style,
    decoration: &Decoration,
    text_field: bool,
) {
    if !clips(style) || decoration.radii.iter().all(|r| *r <= 0.0) {
        return;
    }
    let scrolling_text = text_field && style.overflow.x == Overflow::Scroll;
    if scrolling_text && clear_of_corners(style, decoration) {
        return;
    }
    let fix = if scrolling_text {
        "remove the radius, or give the field side padding at least its corners' radius less its border"
    } else {
        "remove the radius or the clip"
    };
    reader.refuse(
        OVERFLOW_X,
        format!("clips to a box with a border-radius, and rounded clips are refused until a slice needs them (docs/F2c.md); {fix}"),
    );
}

/// Whether a text field's content box, on x, starts and ends clear of
/// its inner corners' curves: each side's padding at least the radius
/// of its corners less that side's border (CSS Backgrounds 3 section
/// 5.2). A percentage padding is not known here, so it is not clear.
fn clear_of_corners(style: &Style, decoration: &Decoration) -> bool {
    let [_, right, _, left] = decoration.border;
    let [top_left, top_right, bottom_right, bottom_left] = decoration.radii;
    let fixed = |side: LengthPercentage| {
        let raw = side.into_raw();
        (raw.tag() == CompactLength::LENGTH_TAG).then(|| raw.value())
    };
    let clear = |padding: LengthPercentage, radius: f32, border: f32| {
        fixed(padding).is_some_and(|padding| padding >= radius - border)
    };
    clear(style.padding.left, top_left.max(bottom_left), left)
        && clear(style.padding.right, top_right.max(bottom_right), right)
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

/// Refuses a number field that scrolls: its text is not scrolled to
/// its caret (`scroll_reveal.rs` reveals a text field's).
pub(crate) fn refuse_scrolling_field(reader: &mut Reader<'_>, style: &Style, number: bool) {
    if number && scrolls(style) {
        reader.refuse(
            OVERFLOW_X,
            "scrolls a number field, which vapors does not do (a text field's text scrolls to its caret, a number field's does not yet); use overflow: hidden or clip".to_owned(),
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
