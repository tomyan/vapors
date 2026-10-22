//! Tokens, CSS Syntax Level 3 shape, and the tokenizer that produces them.

mod consume;
mod cursor;

use crate::{Error, Position, Result};
use cursor::{Cursor, is_whitespace};

/// One token with the position it started at.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    /// What the token is.
    pub kind: TokenKind,
    /// Where it starts in the source.
    pub position: Position,
}

/// Token kinds, CSS Syntax Level 3 section 4.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    /// An identifier, escapes resolved.
    Ident(String),
    /// A function name followed by `(`; the name without the paren.
    Function(String),
    /// `@name`; the name without the `@`.
    AtKeyword(String),
    /// `#value`; `is_id` is true when the value is a valid identifier.
    Hash {
        /// The text after `#`.
        value: String,
        /// Whether it could be an id selector.
        is_id: bool,
    },
    /// A quoted string, quotes removed and escapes resolved.
    String(String),
    /// `url(...)` with an unquoted argument.
    Url(String),
    /// A number; `integer` is true when written without `.` or exponent,
    /// `signed` when it started with `+` or `-` (the An+B grammar needs it).
    Number {
        /// The value.
        value: f64,
        /// Whether it was written as an integer.
        integer: bool,
        /// Whether a sign character was written.
        signed: bool,
    },
    /// A number followed by `%`.
    Percentage(f64),
    /// A number followed by a unit identifier.
    Dimension {
        /// The value.
        value: f64,
        /// The unit, lowercased.
        unit: String,
    },
    /// A run of whitespace.
    Whitespace,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `[`
    OpenSquare,
    /// `]`
    CloseSquare,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `{`
    OpenCurly,
    /// `}`
    CloseCurly,
    /// Any other single character.
    Delim(char),
}

/// Tokenize a stylesheet. Comments and `<!--`/`-->` are dropped; an
/// unterminated comment, string or `url()` is refused at the position it
/// starts.
///
/// # Errors
/// The position and reason for the first construct that cannot be
/// tokenized.
pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    let mut cur = Cursor::new(src);
    let mut tokens = Vec::new();
    while !cur.is_at_end() {
        let position = cur.position();
        if let Some(kind) = next(&mut cur, position)? {
            tokens.push(Token { kind, position });
        }
    }
    Ok(tokens)
}

/// The position just past the end of `src`, where a refusal at the end
/// of input points.
#[must_use]
pub fn end_position(src: &str) -> Position {
    Cursor::new(src).end()
}

/// Consume one token (4.3.1). `None` for a comment or CDO/CDC.
fn next(cur: &mut Cursor, start: Position) -> Result<Option<TokenKind>> {
    if cur.starts_with("/*") {
        comment(cur, start)?;
        return Ok(None);
    }
    if cur.starts_with("<!--") {
        cur.bump_n(4);
        return Ok(None);
    }
    if cur.starts_with("-->") {
        cur.bump_n(3);
        return Ok(None);
    }
    let c = cur.peek(0).expect("caller checked not at end");
    if is_whitespace(c) {
        cur.take_while(is_whitespace);
        return Ok(Some(TokenKind::Whitespace));
    }
    if consume::starts_number(cur) {
        return consume::numeric(cur, start).map(Some);
    }
    if consume::starts_ident(cur) {
        return consume::ident_like(cur, start).map(Some);
    }
    punctuation(cur, c, start).map(Some)
}

fn comment(cur: &mut Cursor, start: Position) -> Result<()> {
    cur.bump_n(2);
    while !cur.is_at_end() {
        if cur.starts_with("*/") {
            cur.bump_n(2);
            return Ok(());
        }
        cur.bump();
    }
    Err(Error::new(start, "unterminated comment; close it with */"))
}

fn punctuation(cur: &mut Cursor, c: char, start: Position) -> Result<TokenKind> {
    cur.bump();
    Ok(match c {
        '"' | '\'' => consume::string(cur, c, start)?,
        '#' => hash(cur)?,
        '@' if consume::starts_ident(cur) => TokenKind::AtKeyword(consume::name(cur)?),
        ':' => TokenKind::Colon,
        ';' => TokenKind::Semicolon,
        ',' => TokenKind::Comma,
        '[' => TokenKind::OpenSquare,
        ']' => TokenKind::CloseSquare,
        '(' => TokenKind::OpenParen,
        ')' => TokenKind::CloseParen,
        '{' => TokenKind::OpenCurly,
        '}' => TokenKind::CloseCurly,
        other => TokenKind::Delim(other),
    })
}

fn hash(cur: &mut Cursor) -> Result<TokenKind> {
    let is_id = consume::starts_ident(cur);
    let has_name = cur.peek(0).is_some_and(cursor::is_name_char) || cur.peek(0) == Some('\\');
    if !has_name {
        return Ok(TokenKind::Delim('#'));
    }
    let value = consume::name(cur)?;
    Ok(TokenKind::Hash { value, is_id })
}
