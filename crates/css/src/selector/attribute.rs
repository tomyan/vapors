//! Attribute selectors: the inside of `[...]`.

use super::parse::Items;
use super::{AttributeOperator, AttributeSelector};
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// The inside of `[...]`.
pub(super) fn parse(contents: &[ComponentValue], at: Position) -> Result<AttributeSelector> {
    let mut items = Items::new(contents, at);
    items.skip_whitespace();
    let name = match items.next() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(n),
            ..
        })) => n.clone(),
        _ => {
            return Err(Error::new(
                at,
                "attribute selector needs a name: [name] or [name=value]",
            ));
        }
    };
    items.skip_whitespace();
    let Some(operator) = operator(&mut items, at)? else {
        return Ok(AttributeSelector {
            name,
            operator: AttributeOperator::Exists,
            value: String::new(),
            case_insensitive: false,
        });
    };
    let value = value(&mut items, &name, at)?;
    let case_insensitive = case_flag(&mut items)?;
    Ok(AttributeSelector {
        name,
        operator,
        value,
        case_insensitive,
    })
}

/// The value after the operator: an identifier or a string.
fn value(items: &mut Items<'_>, name: &str, at: Position) -> Result<String> {
    items.skip_whitespace();
    match items.next() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(v) | TokenKind::String(v),
            ..
        })) => Ok(v.clone()),
        _ => Err(Error::new(
            at,
            format!("attribute selector [{name}...] needs a value after the operator"),
        )),
    }
}

/// An optional `i` (case-insensitive) or `s` flag.
fn case_flag(items: &mut Items<'_>) -> Result<bool> {
    items.skip_whitespace();
    match items.next() {
        None => Ok(false),
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(f),
            ..
        })) if f.eq_ignore_ascii_case("i") => Ok(true),
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(f),
            ..
        })) if f.eq_ignore_ascii_case("s") => Ok(false),
        Some(v) => Err(Error::new(
            v.position(),
            "expected \"i\", \"s\" or \"]\" after the attribute value",
        )),
    }
}

fn operator(items: &mut Items<'_>, at: Position) -> Result<Option<AttributeOperator>> {
    let Some(ComponentValue::Token(Token {
        kind: TokenKind::Delim(c),
        ..
    })) = items.peek()
    else {
        return match items.peek() {
            None => Ok(None),
            Some(v) => Err(Error::new(
                v.position(),
                "expected an attribute operator such as = or ^=",
            )),
        };
    };
    let op = match c {
        '=' => {
            items.next();
            return Ok(Some(AttributeOperator::Equals));
        }
        '^' => AttributeOperator::Prefix,
        '$' => AttributeOperator::Suffix,
        '*' => AttributeOperator::Substring,
        '~' => AttributeOperator::Includes,
        '|' => AttributeOperator::DashMatch,
        other => {
            return Err(Error::new(
                at,
                format!("{other:?} is not an attribute operator"),
            ));
        }
    };
    items.next();
    if !items.eat_delim('=') {
        return Err(Error::new(
            at,
            format!("attribute operator {c} must be followed by ="),
        ));
    }
    Ok(Some(op))
}
