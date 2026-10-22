//! The range form of a media feature: `name op value`, `value op name`
//! and `lo op name op hi`.

use super::media_feature::{Comparison, Feature, SizeAxis, ident, length_px, resolution};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Result};

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
    let parse_value = |v: &ComponentValue| match name.as_str() {
        "resolution" => resolution(v, refuse),
        _ => length_px(v, refuse),
    };
    let mut tests = Vec::new();
    if name_at == 2 {
        // value op name: flip the operator.
        tests.push((
            comparison(&items[..2], refuse)?.flipped(),
            parse_value(items[0])?,
        ));
    } else if name_at != 0 {
        return Err(refuse(
            "a range is `name op value`, `value op name` or `lo op name op hi`",
        ));
    }
    let rest = &items[name_at + 1..];
    if !rest.is_empty() {
        let op_len = if matches!(
            rest.get(1),
            Some(ComponentValue::Token(Token {
                kind: TokenKind::Delim('='),
                ..
            }))
        ) {
            2
        } else {
            1
        };
        if rest.len() != op_len + 1 {
            return Err(refuse(
                "a range is `name op value`, `value op name` or `lo op name op hi`",
            ));
        }
        tests.push((
            comparison(&rest[..op_len], refuse)?,
            parse_value(rest[op_len])?,
        ));
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
