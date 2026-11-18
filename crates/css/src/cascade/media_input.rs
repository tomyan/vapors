//! The shape and input media features: `orientation`, `aspect-ratio`,
//! `pointer` and `hover`.

use super::media_feature::{Comparison, Feature, ident};
use crate::ast::ComponentValue;
use crate::cascade::Pointer;
use crate::token::{Token, TokenKind};
use crate::{Error, Result};

const RATIO: &str =
    "aspect-ratio takes <number> / <number> (both above 0, such as 16/9) or one <number> above 0";

/// `orientation`, `pointer` and `hover` with their one word; `None`
/// when `name` is none of them.
pub(super) fn input(
    name: &str,
    value: &ComponentValue,
    refuse: &dyn Fn(&str) -> Error,
) -> Option<Result<Feature>> {
    let word = ident(Some(value));
    let word = word.as_deref();
    Some(match (name, word) {
        ("orientation", Some("portrait")) => Ok(Feature::Orientation { portrait: true }),
        ("orientation", Some("landscape")) => Ok(Feature::Orientation { portrait: false }),
        ("orientation", _) => Err(refuse("orientation takes portrait or landscape")),
        ("pointer", Some("none")) => Ok(Feature::Pointer(Pointer::None)),
        ("pointer", Some("coarse")) => Ok(Feature::Pointer(Pointer::Coarse)),
        ("pointer", Some("fine")) => Ok(Feature::Pointer(Pointer::Fine)),
        ("pointer", _) => Err(refuse("pointer takes none, coarse or fine")),
        ("hover", Some("none")) => Ok(Feature::Hover(false)),
        ("hover", Some("hover")) => Ok(Feature::Hover(true)),
        ("hover", _) => Err(refuse("hover takes none or hover")),
        _ => return None,
    })
}

/// `aspect-ratio: 16/9` and its `min-`/`max-` forms (`op`).
pub(super) fn aspect_ratio(
    op: Comparison,
    value: &[&ComponentValue],
    refuse: &dyn Fn(&str) -> Error,
) -> Result<Feature> {
    let ratio = match value {
        [n] => number(n),
        [a, d, b] if is_slash(d) => number(a).zip(number(b)).map(|(a, b)| a / b),
        _ => None,
    };
    match ratio {
        Some(r) if r.is_finite() && r > 0.0 => Ok(Feature::AspectRatio(vec![(op, r)])),
        _ => Err(refuse(RATIO)),
    }
}

fn number(v: &ComponentValue) -> Option<f64> {
    match v {
        ComponentValue::Token(Token {
            kind: TokenKind::Number { value, .. },
            ..
        }) => Some(*value),
        _ => None,
    }
}

fn is_slash(v: &ComponentValue) -> bool {
    matches!(
        v,
        ComponentValue::Token(Token {
            kind: TokenKind::Delim('/'),
            ..
        })
    )
}
