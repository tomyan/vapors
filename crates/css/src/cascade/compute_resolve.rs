//! Resolving a specified value to its computed form.

use vapors_scene::Color as Rgba;

use crate::value::{Calc, CalcKind, Color, ColorScheme, Context, Length, LengthUnit, Value};
use crate::{Error, Position, Result};

/// Lengths to px (percentages kept), `calc()` evaluated where it can be,
/// colours to sRGB.
pub(super) fn resolve(
    value: &Value,
    ctx: &Context,
    scheme: ColorScheme,
    current: Rgba,
) -> Result<Value> {
    Ok(match value {
        Value::Length(l) if l.unit != LengthUnit::Percent || ctx.percent_base.is_some() => {
            Value::Length(Length::px(
                l.to_px(ctx)
                    .map_err(|e| Error::new(Position::default(), e.to_string()))?,
            ))
        }
        Value::Calc(c) if c.kind() == CalcKind::Number => Value::Number(evaluate(c, ctx)?),
        Value::Calc(c) if c.kind() == CalcKind::Length || ctx.percent_base.is_some() => {
            Value::Length(Length::px(evaluate(c, ctx)?))
        }
        // A % term stays for layout; every other unit resolves now.
        Value::Calc(c) => Value::Calc(c.with_absolute_lengths(ctx)),
        Value::Color(c) => Value::Color(Color::Rgba(c.resolve(scheme, current))),
        Value::Function(name, args) => {
            Value::Function(name.clone(), resolve_all(args, ctx, scheme, current)?)
        }
        Value::List(items) => Value::List(resolve_all(items, ctx, scheme, current)?),
        Value::CommaList(items) => Value::CommaList(resolve_all(items, ctx, scheme, current)?),
        other => other.clone(),
    })
}

fn resolve_all(
    items: &[Value],
    ctx: &Context,
    scheme: ColorScheme,
    current: Rgba,
) -> Result<Vec<Value>> {
    items
        .iter()
        .map(|v| resolve(v, ctx, scheme, current))
        .collect()
}

fn evaluate(c: &Calc, ctx: &Context) -> Result<f64> {
    c.evaluate(ctx)
        .map_err(|e| Error::new(Position::default(), e.to_string()))
}
