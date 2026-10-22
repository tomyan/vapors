//! The range form of a media feature: `name op value`, `value op name`
//! and `lo op name op hi`.

use super::media_feature::{Comparison, Feature, SizeAxis, ident, length_px, resolution};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Result};

const SHAPE: &str = "a range is `name op value`, `value op name` or `lo op name op hi`";

/// `name op value`, `value op name`, `lo op name op hi`.
pub(super) fn range_form(
    items: &[&ComponentValue],
    refuse: &dyn Fn(&str) -> Error,
) -> Result<Feature> {
    let name_at = items
        .iter()
        .position(|v| ident(Some(v)).is_some())
        .ok_or_else(|| refuse("expected a feature name in the range"))?;
    let name = ident(Some(items[name_at])).expect("found above");
    let mut tests = Vec::new();
    match name_at {
        0 => {}
        // value op name: flip the operator.
        2 => tests.push((
            comparison(&items[..2], refuse)?.flipped(),
            value(&name, items[0], refuse)?,
        )),
        _ => return Err(refuse(SHAPE)),
    }
    let rest = &items[name_at + 1..];
    if !rest.is_empty() {
        tests.push(trailing_test(&name, rest, refuse)?);
    }
    match name.as_str() {
        "width" => Ok(Feature::Size(SizeAxis::Width, tests)),
        "height" => Ok(Feature::Size(SizeAxis::Height, tests)),
        "resolution" => Ok(Feature::Resolution(tests)),
        _ => Err(refuse(
            "only width, height and resolution take the range syntax",
        )),
    }
}

/// `op value` after the name.
fn trailing_test(
    name: &str,
    rest: &[&ComponentValue],
    refuse: &dyn Fn(&str) -> Error,
) -> Result<(Comparison, f64)> {
    let second_is_eq = matches!(
        rest.get(1),
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Delim('='),
            ..
        }))
    );
    let op_len = if second_is_eq { 2 } else { 1 };
    if rest.len() != op_len + 1 {
        return Err(refuse(SHAPE));
    }
    Ok((
        comparison(&rest[..op_len], refuse)?,
        value(name, rest[op_len], refuse)?,
    ))
}

/// The feature's value: dppx for resolution, px otherwise.
fn value(name: &str, v: &ComponentValue, refuse: &dyn Fn(&str) -> Error) -> Result<f64> {
    match name {
        "resolution" => resolution(v, refuse),
        _ => length_px(v, refuse),
    }
}

impl Comparison {
    fn flipped(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::LessEq => Self::GreaterEq,
            Self::Eq => Self::Eq,
            Self::GreaterEq => Self::LessEq,
            Self::Greater => Self::Less,
        }
    }
}

fn comparison(items: &[&ComponentValue], refuse: &dyn Fn(&str) -> Error) -> Result<Comparison> {
    let delims: Vec<char> = items
        .iter()
        .filter_map(|v| match v {
            ComponentValue::Token(Token {
                kind: TokenKind::Delim(c),
                ..
            }) => Some(*c),
            _ => None,
        })
        .collect();
    match delims.as_slice() {
        ['<'] => Ok(Comparison::Less),
        ['<', '='] => Ok(Comparison::LessEq),
        ['='] => Ok(Comparison::Eq),
        ['>', '='] => Ok(Comparison::GreaterEq),
        ['>'] => Ok(Comparison::Greater),
        _ => Err(refuse("expected <, <=, =, >= or >")),
    }
}
