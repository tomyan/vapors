//! `overflow-x` and `overflow-y` (F2c S8): `visible`, and `hidden` and
//! `clip`, which cut the element's descendants off at its padding box.
//! `scroll` and `auto` need a scroll position and are refused until F2c
//! S9. Axes that differ are refused: CSS turns `hidden` beside
//! `visible` into a scroll container (S9), and a one-axis `clip` is not
//! built. A clip with rounded corners is refused until a slice needs it.

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

fn scheme(value: &Value) -> Result<Overflow, String> {
    match keyword(value)? {
        "visible" => Ok(Overflow::Visible),
        "hidden" => Ok(Overflow::Hidden),
        "clip" => Ok(Overflow::Clip),
        "scroll" | "auto" => {
            Err("scrolls, which arrives in F2c S9 (docs/F2c.md); use hidden until then".to_owned())
        }
        other => Err(unknown(other)),
    }
}
