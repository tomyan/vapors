//! The per-token consumers of CSS Syntax Level 3 section 4.3.

use super::TokenKind;
use super::cursor::{Cursor, is_name_char, is_name_start, is_whitespace};
use crate::{Error, Position, Result};

/// Whether the cursor sits on the start of an identifier (4.3.9).
pub(super) fn starts_ident(cur: &Cursor) -> bool {
    match cur.peek(0) {
        Some('-') => match cur.peek(1) {
            Some('-') => true,
            Some(c) if is_name_start(c) => true,
            Some('\\') => valid_escape(cur, 1),
            _ => false,
        },
        Some('\\') => valid_escape(cur, 0),
        Some(c) => is_name_start(c),
        None => false,
    }
}

fn valid_escape(cur: &Cursor, at: usize) -> bool {
    cur.peek(at) == Some('\\') && !matches!(cur.peek(at + 1), None | Some('\n'))
}

/// Whether the cursor sits on the start of a number (4.3.10).
pub(super) fn starts_number(cur: &Cursor) -> bool {
    match cur.peek(0) {
        Some('+' | '-') => match cur.peek(1) {
            Some(c) if c.is_ascii_digit() => true,
            Some('.') => cur.peek(2).is_some_and(|c| c.is_ascii_digit()),
            _ => false,
        },
        Some('.') => cur.peek(1).is_some_and(|c| c.is_ascii_digit()),
        Some(c) => c.is_ascii_digit(),
        None => false,
    }
}

/// Consume a name, resolving escapes (4.3.11).
pub(super) fn name(cur: &mut Cursor) -> Result<String> {
    let mut out = String::new();
    loop {
        match cur.peek(0) {
            Some(c) if is_name_char(c) => {
                out.push(c);
                cur.bump();
            }
            Some('\\') if valid_escape(cur, 0) => {
                cur.bump();
                out.push(escape(cur)?);
            }
            _ => return Ok(out),
        }
    }
}

/// Consume an escaped code point after the backslash (4.3.7).
fn escape(cur: &mut Cursor) -> Result<char> {
    let at = cur.position();
    let hex = cur.take_while(|c| c.is_ascii_hexdigit());
    if hex.is_empty() {
        return cur
            .bump()
            .ok_or_else(|| Error::new(at, "escape at end of input; remove the backslash"));
    }
    if hex.len() > 6 {
        return Err(Error::new(at, "escape has more than six hex digits"));
    }
    if cur.peek(0).is_some_and(is_whitespace) {
        cur.bump();
    }
    let code = u32::from_str_radix(&hex, 16).map_err(|e| Error::new(at, e.to_string()))?;
    Ok(char::from_u32(code).unwrap_or('\u{fffd}'))
}

/// Consume a quoted string after its opening quote (4.3.5).
pub(super) fn string(cur: &mut Cursor, quote: char, start: Position) -> Result<TokenKind> {
    let mut out = String::new();
    loop {
        match cur.bump() {
            None | Some('\n') => {
                return Err(Error::new(
                    start,
                    format!("unterminated string; close it with {quote} before the line ends"),
                ));
            }
            Some(c) if c == quote => return Ok(TokenKind::String(out)),
            Some('\\') => match cur.peek(0) {
                Some('\n') => {
                    cur.bump();
                }
                None => {}
                Some(_) => out.push(escape(cur)?),
            },
            Some(c) => out.push(c),
        }
    }
}

/// Consume a number and whatever makes it a percentage or dimension
/// (4.3.3 and 4.3.12).
pub(super) fn numeric(cur: &mut Cursor, start: Position) -> Result<TokenKind> {
    let (text, integer) = number_text(cur);
    let value: f64 = text
        .parse()
        .map_err(|_| Error::new(start, format!("malformed number {text:?}")))?;
    if cur.peek(0) == Some('%') {
        cur.bump();
        return Ok(TokenKind::Percentage(value));
    }
    if starts_ident(cur) {
        let unit = name(cur)?.to_ascii_lowercase();
        return Ok(TokenKind::Dimension { value, unit });
    }
    Ok(TokenKind::Number { value, integer })
}

fn number_text(cur: &mut Cursor) -> (String, bool) {
    let mut text = String::new();
    let mut integer = true;
    if matches!(cur.peek(0), Some('+' | '-')) {
        text.extend(cur.bump());
    }
    text.push_str(&cur.take_while(|c| c.is_ascii_digit()));
    if cur.peek(0) == Some('.') && cur.peek(1).is_some_and(|c| c.is_ascii_digit()) {
        integer = false;
        cur.bump();
        text.push('.');
        text.push_str(&cur.take_while(|c| c.is_ascii_digit()));
    }
    if exponent_follows(cur) {
        integer = false;
        text.push('e');
        cur.bump();
        if matches!(cur.peek(0), Some('+' | '-')) {
            text.extend(cur.bump());
        }
        text.push_str(&cur.take_while(|c| c.is_ascii_digit()));
    }
    (text, integer)
}

fn exponent_follows(cur: &Cursor) -> bool {
    if !matches!(cur.peek(0), Some('e' | 'E')) {
        return false;
    }
    match cur.peek(1) {
        Some(c) if c.is_ascii_digit() => true,
        Some('+' | '-') => cur.peek(2).is_some_and(|c| c.is_ascii_digit()),
        _ => false,
    }
}

/// Consume an ident-like token: ident, function, or unquoted url (4.3.4).
pub(super) fn ident_like(cur: &mut Cursor, start: Position) -> Result<TokenKind> {
    let text = name(cur)?;
    if cur.peek(0) != Some('(') {
        return Ok(TokenKind::Ident(text));
    }
    cur.bump();
    if text.eq_ignore_ascii_case("url") && !url_is_quoted(cur) {
        return url(cur, start);
    }
    Ok(TokenKind::Function(text))
}

fn url_is_quoted(cur: &Cursor) -> bool {
    let mut i = 0;
    while cur.peek(i).is_some_and(is_whitespace) {
        i += 1;
    }
    matches!(cur.peek(i), Some('"' | '\''))
}

fn url(cur: &mut Cursor, start: Position) -> Result<TokenKind> {
    cur.take_while(is_whitespace);
    let body = cur.take_while(|c| c != ')' && !is_whitespace(c) && c != '"' && c != '\'');
    cur.take_while(is_whitespace);
    if cur.bump() != Some(')') {
        return Err(Error::new(
            start,
            "unterminated url(); close it with ) or quote the argument",
        ));
    }
    Ok(TokenKind::Url(body))
}
