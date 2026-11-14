//! Whether a specified value needs no context to compute: the
//! "constant value" of `build-time-styles.md`, section 1. Every
//! `Value` and `Color` variant is placed, so a new one fails to
//! compile here rather than being judged constant.

use crate::value::{Color, LengthUnit, Value};

/// Whether `value` computes to the same thing under every parent,
/// viewport, root font size and colour scheme.
pub(super) fn is_constant(value: &Value) -> bool {
    match value {
        Value::Length(length) => matches!(length.unit, LengthUnit::Px),
        Value::Color(color) => color_is_constant(color),
        // may hold a relative unit anywhere inside
        Value::Calc(_) => false,
        Value::Function(_, items) | Value::List(items) | Value::CommaList(items) => {
            items.iter().all(is_constant)
        }
        Value::Keyword(_)
        | Value::Number(_)
        | Value::Integer(_)
        | Value::Percentage(_)
        | Value::String(_)
        | Value::Time(_)
        | Value::Flex(_)
        | Value::Ratio(_, _) => true,
    }
}

fn color_is_constant(color: &Color) -> bool {
    match color {
        Color::Rgba(_) => true,
        // the element's `color`; the colour scheme
        Color::CurrentColor | Color::LightDark(_, _) => false,
    }
}
