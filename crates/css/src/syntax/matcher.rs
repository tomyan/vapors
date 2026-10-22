//! Matching component values against a [`Grammar`], with backtracking.

use super::grammar::Grammar;
use super::types;
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::value::Value;

/// Every way `node` can match starting at `start`: the value (None for
/// an optional that matched nothing) and the index after it.
type Matches = Vec<(Option<Value>, usize)>;

/// Match the whole input.
pub(super) fn match_all(grammar: &Grammar, input: &[&ComponentValue]) -> Option<Value> {
    node(grammar, input, 0)
        .into_iter()
        .find(|(_, next)| *next == input.len())
        .and_then(|(v, _)| v)
}

fn node(g: &Grammar, input: &[&ComponentValue], start: usize) -> Matches {
    match g {
        Grammar::Keyword(k) => keyword(k, input, start),
        Grammar::Type { name, range } => types::parse(name, *range, input, start)
            .map(|(v, next)| vec![(Some(v), next)])
            .unwrap_or_default(),
        Grammar::Alternatives(alts) => alts.iter().flat_map(|a| node(a, input, start)).collect(),
        Grammar::Sequence(terms) => sequence(terms, input, start),
        Grammar::Optional(inner) => {
            let mut out = node(inner, input, start);
            out.push((None, start));
            out
        }
        Grammar::Repeat {
            node: inner,
            min,
            max,
        } => repeat(inner, *min, *max, input, start),
        Grammar::CommaList(inner) => comma_list(inner, input, start),
    }
}

fn keyword(k: &str, input: &[&ComponentValue], start: usize) -> Matches {
    match input.get(start) {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(name),
            ..
        })) if name.eq_ignore_ascii_case(k) => {
            vec![(Some(Value::Keyword(k.to_owned())), start + 1)]
        }
        _ => Vec::new(),
    }
}

fn sequence(terms: &[Grammar], input: &[&ComponentValue], start: usize) -> Matches {
    let mut partial: Vec<(Vec<Value>, usize)> = vec![(Vec::new(), start)];
    for term in terms {
        let mut next = Vec::new();
        for (values, at) in &partial {
            for (v, after) in node(term, input, *at) {
                let mut values = values.clone();
                values.extend(v);
                next.push((values, after));
            }
        }
        partial = next;
    }
    partial
        .into_iter()
        .map(|(values, at)| (collapse(values), at))
        .collect()
}

/// Zero values is nothing, one is itself, more is a space list.
fn collapse(mut values: Vec<Value>) -> Option<Value> {
    match values.len() {
        0 => None,
        1 => values.pop(),
        _ => Some(Value::List(values)),
    }
}

fn repeat(
    inner: &Grammar,
    min: usize,
    max: usize,
    input: &[&ComponentValue],
    start: usize,
) -> Matches {
    let mut out = Vec::new();
    collect_repeats(inner, min, max, input, start, Vec::new(), &mut out, None);
    out
}

/// Depth-first over every count of repetitions, longest first.
#[allow(clippy::too_many_arguments)] // one recursion state per argument
fn collect_repeats(
    inner: &Grammar,
    min: usize,
    max: usize,
    input: &[&ComponentValue],
    at: usize,
    so_far: Vec<Value>,
    out: &mut Matches,
    separator: Option<&TokenKind>,
) {
    if so_far.len() < max {
        let next_at = match separator {
            Some(sep) if !so_far.is_empty() => match input.get(at) {
                Some(ComponentValue::Token(Token { kind, .. })) if kind == sep => Some(at + 1),
                _ => None,
            },
            _ => Some(at),
        };
        if let Some(next_at) = next_at {
            for (v, after) in node(inner, input, next_at) {
                if let Some(v) = v {
                    let mut values = so_far.clone();
                    values.push(v);
                    collect_repeats(inner, min, max, input, after, values, out, separator);
                }
            }
        }
    }
    if so_far.len() >= min {
        let value = if separator.is_some() {
            Some(Value::CommaList(so_far))
        } else {
            collapse(so_far)
        };
        out.push((value, at));
    }
}

fn comma_list(inner: &Grammar, input: &[&ComponentValue], start: usize) -> Matches {
    let mut out = Vec::new();
    collect_repeats(
        inner,
        1,
        usize::MAX,
        input,
        start,
        Vec::new(),
        &mut out,
        Some(&TokenKind::Comma),
    );
    out
}
