//! Types with their own small grammars: easing functions, transform and
//! filter functions, shadows, font family names and grid tracks.

// a, b, c, d are cubic-bezier's own argument names.
#![allow(clippy::many_single_char_names)]

use super::special_grid;
use super::types::{self, Matched};
use crate::ast::ComponentValue;
use crate::parse::component::split_commas;
use crate::token::{Token, TokenKind};
use crate::value::Value;

pub(super) fn parse(name: &str, input: &[&ComponentValue], start: usize) -> Matched {
    let first = input.get(start)?;
    let one = |v| Some((v, start + 1));
    match name {
        "easing-function" => one(easing(first)?),
        "transform-function" => one(transform(first)?),
        "filter-function" => one(filter(first)?),
        "font-family-name" => font_family(input, start),
        "shadow" => shadow(input, start),
        "track-size" => one(special_grid::track_size(first)?),
        "track-list" => special_grid::track_list(input, start),
        other => panic!("property table names unknown type <{other}>"),
    }
}

pub(super) fn function(v: &ComponentValue) -> Option<(&str, Vec<Vec<ComponentValue>>)> {
    match v {
        ComponentValue::Function { name, args, .. } => Some((name.as_str(), split_commas(args))),
        _ => None,
    }
}

pub(super) fn single(part: &[ComponentValue]) -> Option<&ComponentValue> {
    match part {
        [v] => Some(v),
        _ => None,
    }
}

pub(super) fn ident(v: &ComponentValue) -> Option<String> {
    match v {
        ComponentValue::Token(Token {
            kind: TokenKind::Ident(s),
            ..
        }) => Some(s.to_ascii_lowercase()),
        _ => None,
    }
}

/// `linear | ease | ease-in | ease-out | ease-in-out | step-start | step-end
/// | cubic-bezier(n, n, n, n) | steps(n [, position]?)`
fn easing(v: &ComponentValue) -> Option<Value> {
    if let Some(k) = ident(v) {
        let names = [
            "linear",
            "ease",
            "ease-in",
            "ease-out",
            "ease-in-out",
            "step-start",
            "step-end",
        ];
        return names.contains(&k.as_str()).then_some(Value::Keyword(k));
    }
    let (name, args) = function(v)?;
    match (name.to_ascii_lowercase().as_str(), args.as_slice()) {
        ("cubic-bezier", [a, b, c, d]) => cubic_bezier([a, b, c, d]),
        ("steps", [n]) => Some(Value::Function(
            "steps".to_owned(),
            vec![types::positive_integer(single(n)?)?],
        )),
        ("steps", [n, pos]) => steps(n, pos),
        _ => None,
    }
}

/// Four numbers; the x control points stay in 0..=1.
fn cubic_bezier(args: [&Vec<ComponentValue>; 4]) -> Option<Value> {
    let nums: Option<Vec<Value>> = args
        .iter()
        .map(|p| types::plain_number(single(p)?))
        .collect();
    let nums = nums?;
    let x_in_range = |n: &Value| matches!(n, Value::Number(x) if (0.0..=1.0).contains(x));
    if !x_in_range(&nums[0]) || !x_in_range(&nums[2]) {
        return None;
    }
    Some(Value::Function("cubic-bezier".to_owned(), nums))
}

/// `steps(n, position)`; jump-none needs two steps to have anywhere to go.
fn steps(n: &[ComponentValue], pos: &[ComponentValue]) -> Option<Value> {
    let count = types::positive_integer(single(n)?)?;
    let pos = ident(single(pos)?)?;
    let known = [
        "start",
        "end",
        "jump-start",
        "jump-end",
        "jump-none",
        "jump-both",
    ];
    if !known.contains(&pos.as_str()) || (pos == "jump-none" && count == Value::Integer(1)) {
        return None;
    }
    Some(Value::Function(
        "steps".to_owned(),
        vec![count, Value::Keyword(pos)],
    ))
}

/// `translate(lp [, lp]?) | translateX(lp) | translateY(lp) | scale(n [, n]?)`
fn transform(v: &ComponentValue) -> Option<Value> {
    let (name, args) = function(v)?;
    let lp = |p: &Vec<ComponentValue>| types::length_percentage(single(p)?);
    let n = |p: &Vec<ComponentValue>| types::plain_number(single(p)?);
    let (name, values) = match (name, args.as_slice()) {
        ("translate", [x]) => ("translate", vec![lp(x)?]),
        ("translate", [x, y]) => ("translate", vec![lp(x)?, lp(y)?]),
        ("translateX", [x]) => ("translateX", vec![lp(x)?]),
        ("translateY", [y]) => ("translateY", vec![lp(y)?]),
        ("scale", [s]) => ("scale", vec![n(s)?]),
        ("scale", [x, y]) => ("scale", vec![n(x)?, n(y)?]),
        _ => return None,
    };
    Some(Value::Function(name.to_owned(), values))
}

/// `blur(<length>)`
fn filter(v: &ComponentValue) -> Option<Value> {
    let (name, args) = function(v)?;
    match (name.to_ascii_lowercase().as_str(), args.as_slice()) {
        ("blur", [l]) => Some(Value::Function(
            "blur".to_owned(),
            vec![types::plain_length(single(l)?)?],
        )),
        _ => None,
    }
}

/// A string, or one or more identifiers joined by spaces.
fn font_family(input: &[&ComponentValue], start: usize) -> Matched {
    if let ComponentValue::Token(Token {
        kind: TokenKind::String(s),
        ..
    }) = input[start]
    {
        return Some((Value::String(s.clone()), start + 1));
    }
    let mut words = Vec::new();
    let mut at = start;
    while let Some(v) = input.get(at) {
        let Some(word) = types::custom_ident(v) else {
            break;
        };
        words.push(word);
        at += 1;
    }
    (!words.is_empty()).then(|| (Value::Keyword(words.join(" ")), at))
}

/// `inset? && <length>{2,4} && <color>?`, any order.
fn shadow(input: &[&ComponentValue], start: usize) -> Matched {
    let mut inset = false;
    let mut lengths = Vec::new();
    let mut color = None;
    let mut at = start;
    while let Some(v) = input.get(at) {
        if ident(v).as_deref() == Some("inset") && !inset {
            inset = true;
        } else if let Some(l) = types::plain_length(v).filter(|_| lengths.len() < 4) {
            lengths.push(l);
        } else if let Some(c) = types::color(v).filter(|_| color.is_none()) {
            color = Some(Value::Color(c));
        } else {
            break;
        }
        at += 1;
    }
    if lengths.len() < 2 {
        return None;
    }
    let mut parts = Vec::new();
    if inset {
        parts.push(Value::Keyword("inset".to_owned()));
    }
    parts.extend(lengths);
    parts.extend(color);
    Some((Value::List(parts), at))
}
