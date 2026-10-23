//! Grid track sizes and track lists, as Taffy 0.15 reads them.

use super::special::{function, ident, single};
use super::types::{self, Matched};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::Value;

/// `<length-percentage> | <flex> | auto | min-content | max-content | minmax(a, b)`
pub(super) fn track_size(v: &ComponentValue) -> Option<Value> {
    if let Some(k) = ident(v) {
        return ["auto", "min-content", "max-content"]
            .contains(&k.as_str())
            .then_some(Value::Keyword(k));
    }
    if let Some(l) = types::length_percentage(v) {
        return Some(l);
    }
    if let ComponentValue::Token(Token {
        kind: TokenKind::Dimension { value, unit },
        ..
    }) = v
        && unit == "fr"
    {
        return Some(Value::Flex(*value));
    }
    let (name, args) = function(v)?;
    match (name.to_ascii_lowercase().as_str(), args.as_slice()) {
        ("minmax", [a, b]) => Some(Value::Function(
            "minmax".to_owned(),
            vec![track_size(single(a)?)?, track_size(single(b)?)?],
        )),
        _ => None,
    }
}

/// `[ <track-size> | repeat(<integer>, <track-size>+) ]+`; line names refused.
pub(super) fn track_list(input: &[&ComponentValue], start: usize) -> Matched {
    let mut items = Vec::new();
    let mut at = start;
    while let Some(v) = input.get(at) {
        if let Some(size) = track_size(v) {
            items.push(size);
        } else if let Some(("repeat", args)) = function(v) {
            let [count, tracks] = args.as_slice() else {
                return None;
            };
            let count = types::positive_integer(single(count)?)?;
            let tracks: Vec<&ComponentValue> =
                tracks.iter().filter(|t| !t.is_whitespace()).collect();
            let (list, end) = track_list(&tracks, 0)?;
            if end != tracks.len() {
                return None;
            }
            items.push(Value::Function("repeat".to_owned(), vec![count, list]));
        } else {
            break;
        }
        at += 1;
    }
    (!items.is_empty()).then_some((Value::List(items), at))
}
