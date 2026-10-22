//! A cursor over a selector's component values.

use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position};

/// A cursor over the prelude.
pub(super) struct Items<'a> {
    values: &'a [ComponentValue],
    index: usize,
    end: Position,
}

impl<'a> Items<'a> {
    pub(super) fn new(values: &'a [ComponentValue], end: Position) -> Self {
        Self {
            values,
            index: 0,
            end,
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

    pub(super) fn position(&self) -> Position {
        self.peek().map_or(self.end, ComponentValue::position)
    }

    pub(super) fn error(&self, message: impl Into<String>) -> Error {
        Error::new(self.position(), message)
    }

    pub(super) fn skip_whitespace(&mut self) -> bool {
        let mut skipped = false;
        while self.peek().is_some_and(ComponentValue::is_whitespace) {
            self.index += 1;
            skipped = true;
        }
        skipped
    }

    pub(super) fn eat_delim(&mut self, c: char) -> bool {
        if matches!(self.peek(), Some(ComponentValue::Token(Token { kind: TokenKind::Delim(d), .. })) if *d == c)
        {
            self.index += 1;
            return true;
        }
        false
    }
}
