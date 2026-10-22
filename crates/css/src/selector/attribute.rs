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
    items.skip_whitespace();
    let value = match items.next() {
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(v) | TokenKind::String(v),
            ..
        })) => v.clone(),
        _ => {
            return Err(Error::new(
                at,
                format!("attribute selector [{name}...] needs a value after the operator"),
            ));
        }
    };
    items.skip_whitespace();
    let case_insensitive = match items.next() {
        None => false,
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(f),
            ..
        })) if f.eq_ignore_ascii_case("i") => true,
        Some(ComponentValue::Token(Token {
            kind: TokenKind::Ident(f),
            ..
        })) if f.eq_ignore_ascii_case("s") => false,
        Some(v) => {
            return Err(Error::new(
                v.position(),
                "expected \"i\", \"s\" or \"]\" after the attribute value",
            ));
        }
    };
    Ok(AttributeSelector {
        name,
        operator,
        value,
        case_insensitive,
    })
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
