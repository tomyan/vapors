//! A cursor over the non-whitespace items of a media query.

use crate::ast::ComponentValue;
use crate::token::{Token, TokenKind};
use crate::{Error, Position};

/// Non-whitespace items of a query with their positions.
pub(super) struct Items<'a> {
    pub(super) values: Vec<&'a ComponentValue>,
    pub(super) index: usize,
    pub(super) at: Position,
}

impl<'a> Items<'a> {
    pub(super) fn new(values: &'a [ComponentValue], at: Position) -> Self {
        Self {
            values: values.iter().filter(|v| !v.is_whitespace()).collect(),
            index: 0,
            at,
        }
    }

    pub(super) fn peek_ident(&self) -> Option<String> {
        match self.values.get(self.index) {
            Some(ComponentValue::Token(Token {
                kind: TokenKind::Ident(s),
                ..
            })) => Some(s.to_ascii_lowercase()),
            _ => None,
        }
    }

    pub(super) fn eat_ident(&mut self, word: &str) -> bool {
        if self.peek_ident().as_deref() == Some(word) {
            self.index += 1;
            return true;
        }
        false
    }

    pub(super) fn position(&self) -> Position {
        self.values
            .get(self.index)
            .map_or(self.at, |v| v.position())
    }

    pub(super) fn error(&self, message: impl Into<String>) -> Error {
        Error::new(self.at, message)
    }
}
