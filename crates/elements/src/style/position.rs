//! `position` and its insets (F2c S8), as Taffy 0.15 takes them: a
//! static box ignores its insets; a relative one is offset by them; an
//! absolute one is placed against its nearest positioned ancestor, and
//! a fixed one against the viewport. `sticky` is refused: it needs the
//! scroll position (S9).

use taffy::{LengthPercentageAuto, Position, Rect, Style};
use vapors_css::value::Value;

use super::box_style::auto_or_amount;
use super::values::{Reader, keyword, unknown};

/// Reads `position` and `top`, `right`, `bottom` and `left` into `style`.
pub(crate) fn position(reader: &mut Reader<'_>, style: &mut Style) {
    style.position = reader.read("position", scheme, Position::Static);
    let auto = LengthPercentageAuto::auto();
    style.inset = Rect {
        top: reader.read("top", auto_or_amount, auto),
        right: reader.read("right", auto_or_amount, auto),
        bottom: reader.read("bottom", auto_or_amount, auto),
        left: reader.read("left", auto_or_amount, auto),
    };
}

fn scheme(value: &Value) -> Result<Position, String> {
    match keyword(value)? {
        "static" => Ok(Position::Static),
        "relative" => Ok(Position::Relative),
        "absolute" => Ok(Position::Absolute),
        "fixed" => Ok(Position::Fixed),
        "sticky" => Err("is sticky, which needs the scroll position; it arrives with scrolling in F2c S9 (docs/F2c.md)".to_owned()),
        other => Err(unknown(other)),
    }
}
