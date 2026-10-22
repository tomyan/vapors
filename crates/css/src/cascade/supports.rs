//! `@supports`: `(property: value)` holds when the property is in the
//! table and the value parses under its grammar; custom properties always
//! hold. `not`, `and`, `or` and parentheses as Conditional Rules Level 3.

use crate::ast::ComponentValue;
use crate::property;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// A parsed `@supports` condition, evaluated at parse time since it
/// depends on nothing but the property table.
#[derive(Clone, Debug)]
pub(crate) struct SupportsCondition(bool);

impl SupportsCondition {
    pub(crate) fn holds(&self) -> bool {
        self.0
    }
}

/// Parse a `@supports` prelude.
pub(crate) fn parse(values: &[ComponentValue]) -> Result<SupportsCondition> {
    let at = values
        .first()
        .map_or(Position::default(), ComponentValue::position);
    let items: Vec<&ComponentValue> = values.iter().filter(|v| !v.is_whitespace()).collect();
    let mut index = 0;
    let result = condition_list(&items, &mut index, at)?;
    if index != items.len() {
        return Err(Error::new(
            at,
            "@supports: unexpected input after the condition",
        ));
    }
    Ok(SupportsCondition(result))
}

fn ident(v: Option<&&ComponentValue>) -> Option<String> {
    match v {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(s),
            ..
        })) => Some(s.to_ascii_lowercase()),
        _ => None,
    }
}

fn condition_list(items: &[&ComponentValue], index: &mut usize, at: Position) -> Result<bool> {
    let first = condition(items, index, at)?;
    let joiner = match ident(items.get(*index)).as_deref() {
        Some("and") => "and",
        Some("or") => "or",
        _ => return Ok(first),
    };
    let mut result = first;
    while ident(items.get(*index)).as_deref() == Some(joiner) {
        *index += 1;
        let next = condition(items, index, at)?;
        result = if joiner == "and" {
            result && next
        } else {
            result || next
        };
    }
    if ident(items.get(*index))
        .as_deref()
        .is_some_and(|w| w == "and" || w == "or")
    {
        return Err(Error::new(
            at,
            "@supports: do not mix `and` and `or` without parentheses",
        ));
    }
    Ok(result)
}

fn condition(items: &[&ComponentValue], index: &mut usize, at: Position) -> Result<bool> {
    if ident(items.get(*index)).as_deref() == Some("not") {
        *index += 1;
        return Ok(!condition(items, index, at)?);
    }
    let Some(ComponentValue::Block {
        open: TokenKind::OpenParen,
        contents,
        ..
    }) = items.get(*index)
    else {
        return Err(Error::new(
            at,
            "@supports: expected a parenthesised condition such as (display: grid)",
        ));
    };
    *index += 1;
    let inner: Vec<&ComponentValue> = contents.iter().filter(|v| !v.is_whitespace()).collect();
    if ident(inner.first()).as_deref() == Some("not")
        || matches!(inner.first(), Some(ComponentValue::Block { .. }))
    {
        let mut inner_index = 0;
        let result = condition_list(&inner, &mut inner_index, at)?;
        if inner_index != inner.len() {
            return Err(Error::new(
                at,
                "@supports: unexpected input inside the parentheses",
            ));
        }
        return Ok(result);
    }
    declaration(contents, at)
}

/// `name: value` holds when the declaration would be accepted.
fn declaration(contents: &[ComponentValue], at: Position) -> Result<bool> {
    let items: Vec<&ComponentValue> = contents.iter().filter(|v| !v.is_whitespace()).collect();
    let (
        Some(name),
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Colon,
            ..
        })),
    ) = (ident(items.first()), items.get(1))
    else {
        return Err(Error::new(at, "@supports: expected (property: value)"));
    };
    let name = property::canonical_name(&name);
    if name.starts_with("--") {
        return Ok(true);
    }
    if !property::is_supported(&name) {
        return Ok(false);
    }
    let colon_at = contents
        .iter()
        .position(|v| {
            matches!(
                v,
                ComponentValue::Token(Token {
                    kind: TokenKind::Colon,
                    ..
                })
            )
        })
        .expect("found above");
    let mut value = contents[colon_at + 1..].to_vec();
    crate::parse::component::trim_whitespace(&mut value);
    Ok(property::parse_declaration(&name, &value, at).is_ok())
}
