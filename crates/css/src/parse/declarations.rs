//! Declaration blocks (sections 5.4.4 and 5.4.6), with property-name
//! checks against the property table.

use super::component::{trim_whitespace, values_until};
use super::stream::{Stream, describe};
use crate::ast::{ComponentValue, Declaration};
use crate::property;
use crate::token::{Token, TokenKind};
use crate::{Error, Result};

/// `{ declarations }`; the stream sits on the `{`.
pub(super) fn block(stream: &mut Stream) -> Result<Vec<Declaration>> {
    let open_at = stream.position();
    stream.expect(&TokenKind::OpenCurly, "\"{\"")?;
    let mut decls = Vec::new();
    loop {
        stream.skip_whitespace();
        match stream.peek() {
            None => {
                return Err(Error::new(
                    open_at,
                    "unterminated declaration block; close it with }",
                ));
            }
            Some(TokenKind::CloseCurly) => {
                stream.next();
                return Ok(decls);
            }
            Some(TokenKind::Semicolon) => {
                stream.next();
            }
            Some(_) => decls.extend(declaration(stream)?),
        }
    }
}

/// One declaration. `None` when the property is on the ignore allowlist.
fn declaration(stream: &mut Stream) -> Result<Option<Declaration>> {
    let position = stream.position();
    let name = match stream.next().map(|t| t.kind) {
        Some(TokenKind::Ident(name)) => name,
        other => {
            return Err(Error::new(
                position,
                format!(
                    "expected a property name, found {}",
                    describe(other.as_ref())
                ),
            ));
        }
    };
    let name = property::canonical_name(&name);
    stream.skip_whitespace();
    if !stream.eat(&TokenKind::Colon) {
        return Err(Error::new(
            position,
            format!(
                "expected \":\" after property {name:?}, found {}",
                describe(stream.peek())
            ),
        ));
    }
    let mut value = values_until(stream, &[TokenKind::Semicolon, TokenKind::CloseCurly])?;
    let important = take_important(&mut value);
    check_value(&value, &name, position)?;
    if property::is_ignored(&name) {
        return Ok(None);
    }
    property::check_name(&name, position)?;
    let longhands = property::parse_declaration(&name, &value, position)?;
    Ok(Some(Declaration {
        name,
        value,
        important,
        longhands,
        position,
    }))
}

/// Strip a trailing `!important` (whitespace allowed after the `!`),
/// reporting whether it was there.
fn take_important(value: &mut Vec<ComponentValue>) -> bool {
    let is_bang = |v: &ComponentValue| {
        matches!(
            v,
            ComponentValue::Token(Token {
                kind: TokenKind::Delim('!'),
                ..
            })
        )
    };
    let is_important = |v: &ComponentValue| matches!(v, ComponentValue::Token(Token { kind: TokenKind::Ident(s), .. }) if s.eq_ignore_ascii_case("important"));
    let Some(last) = value.last() else {
        return false;
    };
    if !is_important(last) {
        return false;
    }
    let bang_at = value[..value.len() - 1]
        .iter()
        .rposition(|v| !v.is_whitespace());
    match bang_at {
        Some(i) if is_bang(&value[i]) => {
            value.truncate(i);
            trim_whitespace(value);
            true
        }
        _ => false,
    }
}

fn check_value(value: &[ComponentValue], name: &str, position: crate::Position) -> Result<()> {
    if value.is_empty() && !name.starts_with("--") {
        return Err(Error::new(
            position,
            format!("property {name:?} has no value"),
        ));
    }
    for v in value {
        refuse_cell_unit(v)?;
    }
    Ok(())
}

/// The `cell` unit is sumi's; vapors lengths are px, em, rem, %, vw, vh, ch.
fn refuse_cell_unit(value: &ComponentValue) -> Result<()> {
    match value {
        ComponentValue::Token(Token {
            kind: TokenKind::Dimension { unit, .. },
            position,
        }) if unit == "cell" => Err(Error::new(
            *position,
            "unit \"cell\" is sumi's terminal cell; vapors lengths use px, em, rem, %, vw, vh or ch",
        )),
        ComponentValue::Function { args, .. } => args.iter().try_for_each(refuse_cell_unit),
        ComponentValue::Block { contents, .. } => contents.iter().try_for_each(refuse_cell_unit),
        ComponentValue::Token(_) => Ok(()),
    }
}
