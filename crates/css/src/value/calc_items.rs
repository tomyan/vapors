//! A cursor over the component values of one `calc()` expression.

use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position};

/// Items of one expression with whitespace tokens dropped but remembered
/// around `+` and `-` (they must be spaced).
pub(super) struct Items<'a> {
    values: &'a [ComponentValue],
    pub(super) index: usize,
    at: Position,
}

impl<'a> Items<'a> {
    pub(super) fn new(values: &'a [ComponentValue], at: Position) -> Self {
        Self {
            values,
            index: 0,
            at,
        }
    }

    pub(super) fn peek(&self) -> Option<&'a ComponentValue> {
        self.values.get(self.index)
    }

    pub(super) fn next(&mut self) -> Option<&'a ComponentValue> {
        let v = self.values.get(self.index)?;
        self.index += 1;
        Some(v)
    }

    pub(super) fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        while self.peek().is_some_and(ComponentValue::is_whitespace) {
            self.index += 1;
            skipped = true;
        }
        skipped
    }

    pub(super) fn peek_delim(&self) -> Option<char> {
        match self.peek() {
            Some(ComponentValue::Token(Token {
                kind: TokenKind::Delim(c),
                ..
            })) => Some(*c),
            _ => None,
        }
    }

    pub(super) fn error(&self, message: impl Into<String>) -> Error {
        Error::new(
            self.peek().map_or(self.at, ComponentValue::position),
            message,
        )
    }
}
