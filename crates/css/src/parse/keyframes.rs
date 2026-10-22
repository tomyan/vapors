//! `@keyframes name { <offset-list> { declarations } ... }`.

use super::component::{split_commas, values_until};
use super::declarations;
use super::stream::{Stream, describe};
use crate::ast::{ComponentValue, KeyframeStop, Keyframes, Stylesheet};
use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

/// Parse the block after the `@keyframes` keyword.
pub(super) fn block(stream: &mut Stream, sheet: &mut Stylesheet, at: Position) -> Result<()> {
    stream.skip_whitespace();
    let name = match stream.peek() {
        Some(TokenKind::Ident(name) | TokenKind::String(name)) => name.clone(),
        other => {
            return Err(Error::new(
                at,
                format!("@keyframes needs a name, found {}", describe(other)),
            ));
        }
    };
    stream.next();
    stream.skip_whitespace();
    let open_at = stream.position();
    stream.expect(
        &TokenKind::OpenCurly,
        &format!("\"{{\" after @keyframes {name}"),
    )?;
    let stops = stops(stream, open_at, &name)?;
    sheet.keyframes.push(Keyframes { name, stops });
    Ok(())
}

fn stops(stream: &mut Stream, open_at: Position, name: &str) -> Result<Vec<KeyframeStop>> {
    let mut stops = Vec::new();
    loop {
        stream.skip_whitespace();
        match stream.peek() {
            None => {
                return Err(Error::new(
                    open_at,
                    format!("unterminated @keyframes {name} block; close it with }}"),
                ));
            }
            Some(TokenKind::CloseCurly) => {
                stream.next();
                return Ok(stops);
            }
            Some(_) => stop(stream, &mut stops)?,
        }
    }
}

fn stop(stream: &mut Stream, stops: &mut Vec<KeyframeStop>) -> Result<()> {
    let prelude = values_until(stream, &[TokenKind::OpenCurly])?;
    let offsets = split_commas(&prelude)
        .iter()
        .map(|part| offset(part, stream.position()))
        .collect::<Result<Vec<f64>>>()?;
    let declarations = declarations::block(stream)?;
    for offset in offsets {
        stops.push(KeyframeStop {
            offset,
            declarations: declarations.clone(),
        });
    }
    Ok(())
}

/// `from`, `to` or a percentage, as a fraction.
fn offset(part: &[ComponentValue], fallback: Position) -> Result<f64> {
    let position = part.first().map_or(fallback, ComponentValue::position);
    let refuse = || {
        Error::new(
            position,
            format!(
                "keyframe selector {:?} is not from, to or a percentage",
                crate::serialize::component_values(part)
            ),
        )
    };
    match part {
        [ComponentValue::Token(Token { kind, .. })] => match kind {
            TokenKind::Ident(s) if s.eq_ignore_ascii_case("from") => Ok(0.0),
            TokenKind::Ident(s) if s.eq_ignore_ascii_case("to") => Ok(1.0),
            TokenKind::Percentage(p) if (0.0..=100.0).contains(p) => Ok(p / 100.0),
            _ => Err(refuse()),
        },
        _ => Err(refuse()),
    }
}
