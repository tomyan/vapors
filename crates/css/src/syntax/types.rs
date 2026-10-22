//! The `<type>` productions: one component value (or a few) to a [`Value`].

use super::special;
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::{Calc, CalcKind, Color, Length, LengthUnit, Time, TimeUnit, Value};

/// A match of one type: the value and the index after it.
pub(super) type Matched = Option<(Value, usize)>;

/// Parse `<name>` at `start`.
pub(super) fn parse(
    name: &str,
    range: Option<(f64, f64)>,
    input: &[&ComponentValue],
    start: usize,
) -> Matched {
    let first = input.get(start)?;
    let one = |v| Some((v, start + 1));
    match name {
        "length" => one(length(first, false, range)?),
        "length-percentage" => one(length(first, true, range)?),
        "percentage" => one(percentage(first, range)?),
        "number" => one(number(first, range)?),
        "integer" => one(integer(first, range)?),
        "color" => one(Value::Color(
            crate::value::parse_color(std::slice::from_ref(first)).ok()?,
        )),
        "string" => one(Value::String(string(first)?)),
        "custom-ident" => one(Value::Keyword(custom_ident(first)?)),
        "time" => one(time(first, range)?),
        "flex" => one(flex(first, range)?),
        "ratio" => ratio(input, start),
        "opentype-tag" => one(Value::String(
            string(first).filter(|s| s.len() == 4 && s.is_ascii())?,
        )),
        other => special::parse(other, input, start),
    }
}

fn token(v: &ComponentValue) -> Option<&TokenKind> {
    match v {
        ComponentValue::Token(Token { kind, .. }) => Some(kind),
        _ => None,
    }
}

fn in_range(n: f64, range: Option<(f64, f64)>) -> bool {
    range.is_none_or(|(lo, hi)| n >= lo && n <= hi)
}

/// A `calc()` whose kind is one of `kinds`.
fn calc(v: &ComponentValue, kinds: &[CalcKind]) -> Option<Calc> {
    if !crate::value::is_math_function(v) {
        return None;
    }
    let c = crate::value::parse_calc(v).ok()?;
    kinds.contains(&c.kind()).then_some(c)
}

fn length(v: &ComponentValue, allow_percent: bool, range: Option<(f64, f64)>) -> Option<Value> {
    let mut kinds = vec![CalcKind::Length];
    if allow_percent {
        kinds.extend([CalcKind::Percentage, CalcKind::LengthPercentage]);
    }
    if let Some(c) = calc(v, &kinds) {
        return Some(Value::Calc(c));
    }
    let l = Length::parse(v).ok()?;
    if l.unit == LengthUnit::Percent && !allow_percent {
        return None;
    }
    in_range(l.value, range).then_some(Value::Length(l))
}

fn percentage(v: &ComponentValue, range: Option<(f64, f64)>) -> Option<Value> {
    if let Some(c) = calc(v, &[CalcKind::Percentage]) {
        return Some(Value::Calc(c));
    }
    match token(v)? {
        TokenKind::Percentage(p) if in_range(*p, range) => Some(Value::Percentage(*p)),
        _ => None,
    }
}

fn number(v: &ComponentValue, range: Option<(f64, f64)>) -> Option<Value> {
    if let Some(c) = calc(v, &[CalcKind::Number]) {
        return Some(Value::Calc(c));
    }
    match token(v)? {
        TokenKind::Number { value, .. } if in_range(*value, range) => Some(Value::Number(*value)),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation)] // the token says it is an integer
fn integer(v: &ComponentValue, range: Option<(f64, f64)>) -> Option<Value> {
    match token(v)? {
        TokenKind::Number {
            value,
            integer: true,
            ..
        } if in_range(*value, range) => Some(Value::Integer(*value as i64)),
        _ => None,
    }
}

fn string(v: &ComponentValue) -> Option<String> {
    match token(v)? {
        TokenKind::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// An identifier that is not a CSS-wide keyword, lowercased.
pub(super) fn custom_ident(v: &ComponentValue) -> Option<String> {
    match token(v)? {
        TokenKind::Ident(s) => {
            let lower = s.to_ascii_lowercase();
            let reserved = ["inherit", "initial", "unset", "revert", "default"];
            (!reserved.contains(&lower.as_str())).then(|| s.clone())
        }
        _ => None,
    }
}

fn time(v: &ComponentValue, range: Option<(f64, f64)>) -> Option<Value> {
    match token(v)? {
        TokenKind::Dimension { value, unit } if in_range(*value, range) => {
            let unit = match unit.as_str() {
                "ms" => TimeUnit::Ms,
                "s" => TimeUnit::S,
                _ => return None,
            };
            Some(Value::Time(Time {
                value: *value,
                unit,
            }))
        }
        _ => None,
    }
}

fn flex(v: &ComponentValue, range: Option<(f64, f64)>) -> Option<Value> {
    match token(v)? {
        TokenKind::Dimension { value, unit } if unit == "fr" && in_range(*value, range) => {
            Some(Value::Flex(*value))
        }
        _ => None,
    }
}

/// `<number> [/ <number>]?`
fn ratio(input: &[&ComponentValue], start: usize) -> Matched {
    let Some(Value::Number(w)) = number(input.get(start)?, Some((0.0, f64::INFINITY))) else {
        return None;
    };
    let slash = matches!(
        input.get(start + 1).and_then(|v| token(v)),
        Some(TokenKind::Delim('/'))
    );
    if !slash {
        return Some((Value::Ratio(w, 1.0), start + 1));
    }
    let Some(Value::Number(h)) = number(input.get(start + 2)?, Some((0.0, f64::INFINITY))) else {
        return None;
    };
    Some((Value::Ratio(w, h), start + 3))
}

/// A colour keyword or value, for the special types.
pub(super) fn color(v: &ComponentValue) -> Option<Color> {
    crate::value::parse_color(std::slice::from_ref(v)).ok()
}

/// A non-negative length-percentage, for the special types.
pub(super) fn length_percentage(v: &ComponentValue) -> Option<Value> {
    length(v, true, None)
}

/// A length, for the special types.
pub(super) fn plain_length(v: &ComponentValue) -> Option<Value> {
    length(v, false, None)
}

/// A number, for the special types.
pub(super) fn plain_number(v: &ComponentValue) -> Option<Value> {
    number(v, None)
}

/// A positive integer, for the special types.
pub(super) fn positive_integer(v: &ComponentValue) -> Option<Value> {
    integer(v, Some((1.0, f64::INFINITY)))
}
