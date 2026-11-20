//! `accent-color` (CSS UI 4): `auto` or a colour, inherited. A checked
//! switch's track is drawn in it; `auto` is the user agent's blue
//! (`parts::DEFAULT`'s mark colour).

use vapors_css::property::longhands::ACCENT_COLOR;
use vapors_css::value::Value;
use vapors_scene::Color;

use super::text_style::color;
use super::values::{Reader, keyword};

/// The colour `accent-color` names, `None` for `auto`.
pub(crate) fn accent(reader: &mut Reader<'_>) -> Option<Color> {
    reader.read(ACCENT_COLOR, accent_value, None)
}

fn accent_value(value: &Value) -> Result<Option<Color>, String> {
    if matches!(keyword(value), Ok("auto")) {
        return Ok(None);
    }
    color(value).map(Some)
}
