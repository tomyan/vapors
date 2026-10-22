//! Shorthand expansion into longhands; the list is in `shorthand_table`.

pub use super::shorthand_table::SHORTHANDS;

use super::declaration::{Longhand, Specified, parse_longhand};
use super::{PropertyId, shorthand_border, shorthand_layout, shorthand_motion};
use crate::ast::ComponentValue;
use crate::value::Value;
use crate::{Error, Position, Result};

/// Whether `name` is a shorthand.
#[must_use]
pub fn is_shorthand(name: &str) -> bool {
    SHORTHANDS.iter().any(|(n, _)| *n == name)
}

/// The longhands of a shorthand.
pub(super) fn longhands_of(name: &str) -> Vec<PropertyId> {
    SHORTHANDS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, longhands)| {
            longhands
                .iter()
                .map(|l| PropertyId::from_name(l).expect("table lists it"))
                .collect()
        })
        .unwrap_or_default()
}

/// Expand a shorthand's value.
pub(super) fn expand(name: &str, values: &[ComponentValue], at: Position) -> Result<Vec<Longhand>> {
    let ids = longhands_of(name);
    match name {
        "margin" | "padding" | "inset" | "border-width" | "border-style" | "border-color"
        | "border-radius" => sides(name, &ids, values, at),
        "gap" | "overflow" => pair(name, &ids, values, at),
        "flex" => shorthand_layout::flex(&ids, values, at),
        "grid-row" | "grid-column" => shorthand_layout::grid_line(name, &ids, values, at),
        "border" => shorthand_border::expand(&ids, values, at),
        "transition" => shorthand_motion::transition(&ids, values, at),
        "animation" => shorthand_motion::animation(&ids, values, at),
        _ => unreachable!("SHORTHANDS lists only expanded names"),
    }
}

/// Split on top-level whitespace.
pub(super) fn parts(values: &[ComponentValue]) -> Vec<Vec<ComponentValue>> {
    let mut parts = vec![Vec::new()];
    for v in values {
        if v.is_whitespace() {
            parts.push(Vec::new());
        } else {
            parts.last_mut().expect("one part").push(v.clone());
        }
    }
    parts.retain(|p| !p.is_empty());
    parts
}

pub(super) fn set(id: PropertyId, value: Value) -> Longhand {
    Longhand {
        id,
        value: Specified::Value(value),
    }
}

/// One to four values: top, right, bottom, left (corners for radius).
fn sides(
    name: &str,
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    if name == "border-radius" && values.iter().any(|v| matches!(v, ComponentValue::Token(t) if t.kind == crate::token::TokenKind::Delim('/'))) {
        return Err(Error::new(at, "border-radius with / (elliptical corners) is not supported; give one radius per corner"));
    }
    let parts = parts(values);
    let order: &[usize] = match parts.len() {
        1 => &[0, 0, 0, 0],
        2 => &[0, 1, 0, 1],
        3 => &[0, 1, 2, 1],
        4 => &[0, 1, 2, 3],
        n => {
            return Err(Error::new(
                at,
                format!("{name} takes one to four values, found {n}"),
            ));
        }
    };
    let parsed: Vec<Value> = parts
        .iter()
        .enumerate()
        .map(|(i, p)| parse_longhand(ids[i], p, at))
        .collect::<Result<_>>()?;
    Ok(ids
        .iter()
        .zip(order)
        .map(|(id, i)| set(*id, parsed[*i].clone()))
        .collect())
}

/// One or two values: first, second (second defaults to first).
fn pair(
    name: &str,
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let parts = parts(values);
    let (a, b) = match parts.as_slice() {
        [a] => (a, a),
        [a, b] => (a, b),
        _ => {
            return Err(Error::new(
                at,
                format!("{name} takes one or two values, found {}", parts.len()),
            ));
        }
    };
    Ok(vec![
        set(ids[0], parse_longhand(ids[0], a, at)?),
        set(ids[1], parse_longhand(ids[1], b, at)?),
    ])
}
