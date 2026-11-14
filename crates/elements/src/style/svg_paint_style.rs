//! An element's SVG paint (docs/plans/one-scene.md 1.5): its computed
//! `fill`, `stroke` and `stroke-width`, with `currentcolor` resolved by
//! the element's own `color`, which `fill` and `stroke` keep as the
//! keyword so a path takes its own (Chromium).

use vapors_css::property::longhands::{FILL, STROKE, STROKE_WIDTH};
use vapors_css::value::{Color, LengthUnit, Value};
use vapors_scene::Color as Rgba;

use super::values::{Reader, shape};

/// `fill`, `stroke` and `stroke-width` as paint reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SvgPaint {
    /// The fill colour; `None` for `none`.
    pub(crate) fill: Option<Rgba>,
    /// The stroke colour; `None` for `none`.
    pub(crate) stroke: Option<Rgba>,
    /// The stroke width in user units (the viewBox's), as a number or
    /// px.
    pub(crate) stroke_width: f32,
}

/// The element's SVG paint, `currentcolor` resolving to `color`.
pub(crate) fn svg_paint(reader: &mut Reader<'_>, color: Rgba) -> SvgPaint {
    let paint = |value: &Value| self::paint(value, color);
    SvgPaint {
        fill: reader.read(FILL, paint, None),
        stroke: reader.read(STROKE, paint, None),
        stroke_width: reader.read(STROKE_WIDTH, stroke_width, 1.0),
    }
}

fn paint(value: &Value, color: Rgba) -> Result<Option<Rgba>, String> {
    match value {
        Value::Keyword(k) if k == "none" => Ok(None),
        Value::Color(Color::Rgba(rgba)) => Ok(Some(*rgba)),
        Value::Color(Color::CurrentColor) => Ok(Some(color)),
        _ => Err(shape("`none`, a colour or `currentcolor`")),
    }
}

#[allow(clippy::cast_possible_truncation)] // a stroke width is far below f32's range
fn stroke_width(value: &Value) -> Result<f32, String> {
    match value {
        Value::Number(n) => Ok(*n as f32),
        Value::Length(length) if length.unit == LengthUnit::Px => Ok(length.value as f32),
        _ => Err(shape("a number or a length in px")),
    }
}
