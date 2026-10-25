//! `scrollbar-color` and `scrollbar-width` (F2c S9): how a scroll
//! container's overlay thumb is drawn. vapors draws only a thumb over
//! the content, taking no space, so `scrollbar-color`'s track colour
//! must be transparent; another is refused rather than not drawn.

use vapors_css::value::Value;
use vapors_scene::Color;

use super::text_style::color;
use super::values::{Reader, keyword, shape, unknown};

/// The thumb's colour where `scrollbar-color` is `auto`. The oracle
/// hides Chromium's scrollbars, so these user-agent values are chosen,
/// not measured: a dark thumb at half strength, readable on light and
/// mid backgrounds.
pub(crate) const AUTO_THUMB: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.5,
};
/// The thumb's thickness (CSS px) for `scrollbar-width: auto`.
pub(crate) const AUTO_WIDTH: f32 = 8.0;
/// The thumb's thickness (CSS px) for `scrollbar-width: thin`.
pub(crate) const THIN_WIDTH: f32 = 4.0;

/// How a scroll container's thumb is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Scrollbar {
    pub(crate) thumb: Color,
    /// Thickness in CSS px; 0 for `none`, which draws no thumb.
    pub(crate) width: f32,
}

/// Reads both properties.
pub(crate) fn scrollbar(reader: &mut Reader<'_>) -> Scrollbar {
    Scrollbar {
        thumb: reader.read("scrollbar-color", thumb, AUTO_THUMB),
        width: reader.read("scrollbar-width", width, AUTO_WIDTH),
    }
}

fn thumb(value: &Value) -> Result<Color, String> {
    match value {
        Value::Keyword(k) if k == "auto" => Ok(AUTO_THUMB),
        Value::List(colors) if colors.len() == 2 => {
            let track = color(&colors[1])?;
            if track.a > 0.0 {
                return Err("has a track colour; vapors draws an overlay thumb with no track, so give the track `transparent`".to_owned());
            }
            color(&colors[0])
        }
        _ => Err(shape("`auto` or a thumb and a track colour")),
    }
}

fn width(value: &Value) -> Result<f32, String> {
    match keyword(value)? {
        "auto" => Ok(AUTO_WIDTH),
        "thin" => Ok(THIN_WIDTH),
        "none" => Ok(0.0),
        other => Err(unknown(other)),
    }
}
