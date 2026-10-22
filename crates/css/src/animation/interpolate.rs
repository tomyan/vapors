//! Interpolation between two computed values at progress `t`.

use vapors_scene::Color as Rgba;

use crate::value::{Color, Length, Value};

/// The value `t` of the way from `from` to `to`. Lengths of one unit,
/// numbers, integers, percentages and colours (in Oklab, premultiplied
/// alpha) interpolate; lists and functions of the same shape do so
/// element-wise; keywords and other same-shaped values snap at 0.5.
/// `None` when the two have no common shape.
#[must_use]
pub fn interpolate(from: &Value, to: &Value, t: f64) -> Option<Value> {
    let snap = || Some(if t < 0.5 { from.clone() } else { to.clone() });
    match (from, to) {
        (Value::Length(a), Value::Length(b)) if a.unit == b.unit => Some(Value::Length(Length {
            value: lerp(a.value, b.value, t),
            unit: a.unit,
        })),
        (Value::Number(a), Value::Number(b)) => Some(Value::Number(lerp(*a, *b, t))),
        (Value::Percentage(a), Value::Percentage(b)) => Some(Value::Percentage(lerp(*a, *b, t))),
        (Value::Flex(a), Value::Flex(b)) => Some(Value::Flex(lerp(*a, *b, t))),
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        // z-index sized integers
        (Value::Integer(a), Value::Integer(b)) => {
            Some(Value::Integer(lerp(*a as f64, *b as f64, t).round() as i64))
        }
        (Value::Color(Color::Rgba(a)), Value::Color(Color::Rgba(b))) => {
            Some(Value::Color(Color::Rgba(color(*a, *b, t))))
        }
        (Value::Function(name, a), Value::Function(other, b))
            if name == other && a.len() == b.len() =>
        {
            Some(Value::Function(name.clone(), elementwise(a, b, t)?))
        }
        (Value::List(a), Value::List(b)) if a.len() == b.len() => {
            Some(Value::List(elementwise(a, b, t)?))
        }
        (Value::CommaList(a), Value::CommaList(b)) if a.len() == b.len() => {
            Some(Value::CommaList(elementwise(a, b, t)?))
        }
        (Value::Keyword(_), Value::Keyword(_))
        | (Value::String(_), Value::String(_))
        | (Value::Time(_), Value::Time(_))
        | (Value::Ratio(..), Value::Ratio(..))
        | (Value::Calc(_), Value::Calc(_))
        | (Value::Length(_), Value::Length(_)) => snap(),
        _ => None,
    }
}

fn elementwise(a: &[Value], b: &[Value], t: f64) -> Option<Vec<Value>> {
    a.iter().zip(b).map(|(x, y)| interpolate(x, y, t)).collect()
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Oklab with premultiplied alpha (Color Level 4 section 12).
fn color(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let la = Color::to_oklab(a);
    let lb = Color::to_oklab(b);
    let alpha = lerp(la[3], lb[3], t);
    if alpha <= 0.0 {
        return Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        };
    }
    let channel = |i: usize| lerp(la[i] * la[3], lb[i] * lb[3], t) / alpha;
    Color::from_oklab([channel(0), channel(1), channel(2), alpha])
}
