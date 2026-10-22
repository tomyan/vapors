//! A cursor over tokens with position bookkeeping for error messages.

use crate::token::{Token, TokenKind};
use crate::{Error, Position, Result};

pub(crate) struct Stream {
    tokens: Vec<Token>,
    index: usize,
    end: Position,
}

impl Stream {
    pub(crate) fn new(tokens: Vec<Token>) -> Self {
        let end = tokens
            .last()
            .map_or(Position { line: 1, column: 1 }, |t| t.position);
        Self {
            tokens,
            index: 0,
            end,
        }
    }

    pub(super) fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.index).map(|t| &t.kind)
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }

    /// The position of the next token, or of the last one at the end.
    pub(super) fn position(&self) -> Position {
        self.tokens.get(self.index).map_or(self.end, |t| t.position)
    }

    pub(super) fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.index).cloned()?;
        self.index += 1;
        Some(t)
    }

    pub(super) fn skip_whitespace(&mut self) {
        while self.peek() == Some(&TokenKind::Whitespace) {
            self.index += 1;
        }
    }

    /// Consume the next token if it is `kind`.
    pub(super) fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.peek() == Some(kind) {
            self.index += 1;
            return true;
        }
        false
    }

    /// Refuse at the current position.
    pub(super) fn error(&self, message: impl Into<String>) -> Error {
        Error::new(self.position(), message)
    }

    /// Consume `kind` or refuse with `expected` in the message.
    pub(super) fn expect(&mut self, kind: &TokenKind, expected: &str) -> Result<()> {
        if self.eat(kind) {
            return Ok(());
        }
        Err(self.error(format!(
            "expected {expected}, found {}",
            describe(self.peek())
        )))
    }
}

/// A token for an error message.
pub(super) fn describe(kind: Option<&TokenKind>) -> String {
    let Some(kind) = kind else {
        return "end of stylesheet".to_owned();
    };
    let mut text = String::new();
    crate::serialize::token(kind, &mut text);
    match kind {
        TokenKind::Whitespace => "whitespace".to_owned(),
        TokenKind::Ident(_) => format!("identifier {text:?}"),
        TokenKind::String(_) => format!("string {text}"),
        _ => format!("{text:?}"),
    }
}
