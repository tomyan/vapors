//! The range form of a media feature (Media Queries Level 4 section
//! 2.4.3): `name op value`, `value op name` and `lo op name op hi`, with
//! `<`, `<=`, `=`, `>=`, `>` written as one or two delimiters.

use super::media_feature::{Comparison, Feature, SizeAxis, ident, length, resolution};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::Length;
use crate::{Error, Result};

const SHAPE: &str = "a range is `name op value`, `value op name` or `lo op name op hi`";

/// One side's comparison, normalised to "name op value".
enum Bound {
    Length(Comparison, Length),
    Resolution(Comparison, f64),
}

/// Parse the range form.
pub(super) fn range_form(
    items: &[&ComponentValue],
    refuse: &dyn Fn(&str) -> Error,
) -> Result<Feature> {
    let name_at = items
        .iter()
        .position(|v| ident(Some(v)).is_some())
        .ok_or_else(|| refuse("expected a feature name in the range"))?;
    let name = ident(Some(items[name_at])).expect("found above");
    let mut bounds = Vec::new();
    if name_at > 0 {
        // `value op name`: the operator reads the other way round.
        let (value, ops) = items[..name_at]
            .split_first()
            .ok_or_else(|| refuse(SHAPE))?;
        bounds.push(bound(
            &name,
            comparison(ops, refuse)?.flipped(),
            value,
            refuse,
        )?);
    }
    let rest = &items[name_at + 1..];
    if let Some((value, ops)) = rest.split_last() {
        bounds.push(bound(&name, comparison(ops, refuse)?, value, refuse)?);
    }
    if bounds.is_empty() {
        return Err(refuse(SHAPE));
    }
    feature(&name, bounds, refuse)
}

fn bound(
    name: &str,
    op: Comparison,
    value: &ComponentValue,
    refuse: &dyn Fn(&str) -> Error,
) -> Result<Bound> {
    Ok(match name {
        "resolution" => Bound::Resolution(op, resolution(value, refuse)?),
        _ => Bound::Length(op, length(value, refuse)?),
    })
}

fn feature(name: &str, bounds: Vec<Bound>, refuse: &dyn Fn(&str) -> Error) -> Result<Feature> {
    let axis = match name {
        "width" => SizeAxis::Width,
        "height" => SizeAxis::Height,
        "resolution" => {
            let tests = bounds
                .into_iter()
                .map(|b| match b {
                    Bound::Resolution(op, v) => (op, v),
                    Bound::Length(..) => unreachable!("bound() keyed on the name"),
                })
                .collect();
            return Ok(Feature::Resolution(tests));
        }
        _ => {
            return Err(refuse(
                "only width, height and resolution take the range syntax",
            ));
        }
    };
    let tests = bounds
        .into_iter()
        .map(|b| match b {
            Bound::Length(op, l) => (op, l),
            Bound::Resolution(..) => unreachable!("bound() keyed on the name"),
        })
        .collect();
    Ok(Feature::Size(axis, tests))
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

/// `<`, `<=`, `=`, `>=` or `>` from one or two delimiter tokens.
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
    if delims.len() != items.len() {
        return Err(refuse(SHAPE));
    }
    match delims.as_slice() {
        ['<'] => Ok(Comparison::Less),
        ['<', '='] => Ok(Comparison::LessEq),
        ['='] => Ok(Comparison::Eq),
        ['>', '='] => Ok(Comparison::GreaterEq),
        ['>'] => Ok(Comparison::Greater),
        _ => Err(refuse("expected <, <=, =, >= or >")),
    }
}
