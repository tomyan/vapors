//! Component values: tokens, functions and blocks (section 5.4.7).

use super::stream::{Stream, describe};
use crate::Result;
use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};

/// Consume one component value. The stream must not be at the end.
pub(super) fn component_value(stream: &mut Stream) -> Result<ComponentValue> {
    let token = stream.next().expect("caller checked for a token");
    match token.kind {
        TokenKind::Function(name) => {
            let args = until_close(stream, &TokenKind::CloseParen, token.position, "function")?;
            Ok(ComponentValue::Function {
                name,
                args,
                position: token.position,
            })
        }
        TokenKind::OpenSquare => block(stream, token, &TokenKind::CloseSquare, "]"),
        TokenKind::OpenParen => block(stream, token, &TokenKind::CloseParen, ")"),
        TokenKind::OpenCurly => block(stream, token, &TokenKind::CloseCurly, "}"),
        _ => Ok(ComponentValue::Token(token)),
    }
}

fn block(
    stream: &mut Stream,
    open: Token,
    close: &TokenKind,
    close_text: &str,
) -> Result<ComponentValue> {
    let contents = until_close(stream, close, open.position, close_text)?;
    Ok(ComponentValue::Block {
        open: open.kind,
        contents,
        position: open.position,
    })
}

/// Consume component values up to and including `close`, refusing at
/// `opened_at` if the input ends first.
fn until_close(
    stream: &mut Stream,
    close: &TokenKind,
    opened_at: crate::Position,
    what: &str,
) -> Result<Vec<ComponentValue>> {
    let mut out = Vec::new();
    loop {
        if stream.is_at_end() {
            let mut close_text = String::new();
            crate::serialize::token(close, &mut close_text);
            return Err(crate::Error::new(
                opened_at,
                format!("unterminated {what}; close it with {close_text}"),
            ));
        }
        if stream.eat(close) {
            return Ok(out);
        }
        out.push(component_value(stream)?);
    }
}

/// Consume component values until one of `stop` appears at the top level
/// (not consumed) or the input ends. Leading and trailing whitespace are
/// trimmed.
pub(super) fn values_until(stream: &mut Stream, stop: &[TokenKind]) -> Result<Vec<ComponentValue>> {
    let mut out = Vec::new();
    while let Some(kind) = stream.peek() {
        if stop.contains(kind) {
            break;
        }
        if is_stray_close(kind) {
            return Err(stream.error(format!("unexpected {}", describe(Some(kind)))));
        }
        out.push(component_value(stream)?);
    }
    trim_whitespace(&mut out);
    Ok(out)
}

fn is_stray_close(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::CloseCurly | TokenKind::CloseParen | TokenKind::CloseSquare
    )
}

/// Remove leading and trailing whitespace tokens.
pub(super) fn trim_whitespace(values: &mut Vec<ComponentValue>) {
    while values.last().is_some_and(ComponentValue::is_whitespace) {
        values.pop();
    }
    let lead = values.iter().take_while(|v| v.is_whitespace()).count();
    values.drain(..lead);
}

/// Split component values on top-level commas, each part trimmed.
pub(super) fn split_commas(values: &[ComponentValue]) -> Vec<Vec<ComponentValue>> {
    let mut parts = vec![Vec::new()];
    for v in values {
        if matches!(
            v,
            ComponentValue::Token(Token {
                kind: TokenKind::Comma,
                ..
            })
        ) {
            parts.push(Vec::new());
        } else {
            parts.last_mut().expect("one part").push(v.clone());
        }
    }
    for p in &mut parts {
        trim_whitespace(p);
    }
    parts
}
