//! `{...}` holes: reading one with balanced braces (skipping string
//! and character literals), and telling a block keyword from an
//! expression.

use super::Parser;
use crate::CompileError;

/// A hole read from the template, not yet consumed.
pub(crate) struct Hole {
    /// Byte offset of the `{`.
    pub offset: usize,
    /// Bytes from the `{` through the `}`.
    pub len: usize,
    /// The content, trimmed.
    pub body: String,
    /// Byte offset of `body` in the file.
    pub body_offset: usize,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Kind {
    Expr,
    If,
    For,
    Else,
    ElseIf,
    Close(String),
    Snippet,
    Render,
    Slot,
}

impl Parser<'_> {
    /// Reads the hole at the cursor without consuming it.
    pub fn peek_hole(&self) -> Result<Hole, CompileError> {
        let offset = self.pos;
        let rest = self.rest();
        let inner_len = balanced_len(&rest[1..]).ok_or_else(|| {
            self.error(
                offset,
                "unterminated `{`: add the matching `}` after the expression",
            )
        })?;
        let raw = &rest[1..=inner_len];
        let body = raw.trim();
        if body.is_empty() {
            return Err(self.error(offset, "empty `{}`: put an expression inside, or remove it"));
        }
        let body_offset = offset + 1 + (raw.len() - raw.trim_start().len());
        Ok(Hole {
            offset,
            len: inner_len + 2,
            body: body.to_string(),
            body_offset,
            kind: classify(body),
        })
    }
}

/// Bytes up to the `}` that closes an already-opened `{`, skipping
/// nested braces and string or character literals.
fn balanced_len(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' => i += string_len(&text[i..])?,
            b'\'' => i += char_literal_len(&text[i..]).unwrap_or(0),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Bytes from the opening quote to the closing one, inclusive.
fn string_len(text: &str) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in text.char_indices().skip(1) {
        match c {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => return Some(i),
            _ => escaped = false,
        }
    }
    None
}

/// `'x'` or `'\n'` is a character literal; `'a` (a lifetime) is not.
fn char_literal_len(text: &str) -> Option<usize> {
    let mut chars = text.char_indices().skip(1);
    let (_, first) = chars.next()?;
    let (_, second) = chars.next()?;
    if first == '\\' {
        let (end, quote) = chars.next()?;
        return (quote == '\'').then_some(end);
    }
    (second == '\'').then_some(first.len_utf8() + 1)
}

fn classify(body: &str) -> Kind {
    if let Some(rest) = body.strip_prefix('/') {
        return Kind::Close(rest.trim().to_string());
    }
    let keyword = body
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .map_or(body, |end| &body[..end]);
    match keyword {
        "if" => Kind::If,
        "for" => Kind::For,
        "else" if body == "else" => Kind::Else,
        "else" if body[4..].trim_start().starts_with("if") => Kind::ElseIf,
        "snippet" => Kind::Snippet,
        "render" => Kind::Render,
        "slot" => Kind::Slot,
        _ => Kind::Expr,
    }
}
