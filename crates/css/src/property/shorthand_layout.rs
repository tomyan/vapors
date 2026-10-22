//! The `flex` and `grid-row`/`grid-column` shorthands.

use super::PropertyId;
use super::declaration::{Longhand, parse_longhand, refusal};
use super::shorthand::{parts, set};
use crate::ast::ComponentValue;
use crate::value::Value;
use crate::{Error, Position, Result};

/// `none | auto | <grow> [<shrink>]? [<basis>]? | <basis>`
pub(super) fn flex(
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let parts = parts(values);
    let number = |p: &[ComponentValue]| {
        crate::syntax::match_value(&crate::syntax::parse_grammar("<number [0,inf]>"), p)
    };
    let basis = |p: &[ComponentValue]| parse_longhand(ids[2], p, at).ok();
    let zero = Value::Length(crate::value::Length::px(0.0));
    let auto = Value::Keyword("auto".to_owned());
    let (grow, shrink, basis) = match parts.as_slice() {
        [k] if crate::serialize::component_values(k) == "none" => {
            (Value::Number(0.0), Value::Number(0.0), auto)
        }
        [k] if crate::serialize::component_values(k) == "auto" => {
            (Value::Number(1.0), Value::Number(1.0), auto)
        }
        [g] if number(g).is_some() => (number(g).expect("checked"), Value::Number(1.0), zero),
        [b] if basis(b).is_some() => (
            Value::Number(1.0),
            Value::Number(1.0),
            basis(b).expect("checked"),
        ),
        [g, s] if number(g).is_some() && number(s).is_some() => (
            number(g).expect("checked"),
            number(s).expect("checked"),
            zero,
        ),
        [g, b] if number(g).is_some() && basis(b).is_some() => (
            number(g).expect("checked"),
            Value::Number(1.0),
            basis(b).expect("checked"),
        ),
        [g, s, b] if number(g).is_some() && number(s).is_some() && basis(b).is_some() => (
            number(g).expect("checked"),
            number(s).expect("checked"),
            basis(b).expect("checked"),
        ),
        _ => {
            return Err(refusal(
                "flex",
                "none | auto | <number> <number>? <flex-basis>?",
                values,
                at,
            ));
        }
    };
    Ok(vec![
        set(ids[0], grow),
        set(ids[1], shrink),
        set(ids[2], basis),
    ])
}

/// `<grid-line> [/ <grid-line>]?`
pub(super) fn grid_line(
    name: &str,
    ids: &[PropertyId],
    values: &[ComponentValue],
    at: Position,
) -> Result<Vec<Longhand>> {
    let mut halves: Vec<Vec<ComponentValue>> = vec![Vec::new()];
    for v in values {
        if matches!(v, ComponentValue::Token(t) if t.kind == crate::token::TokenKind::Delim('/')) {
            halves.push(Vec::new());
        } else {
            halves.last_mut().expect("one").push(v.clone());
        }
    }
    let start = parse_longhand(ids[0], &halves[0], at)?;
    let end = match halves.get(1) {
        Some(h) => parse_longhand(ids[1], h, at)?,
        None => Value::Keyword("auto".to_owned()),
    };
    if halves.len() > 2 {
        return Err(Error::new(
            at,
            format!("{name} takes at most two lines separated by /"),
        ));
    }
    Ok(vec![set(ids[0], start), set(ids[1], end)])
}
