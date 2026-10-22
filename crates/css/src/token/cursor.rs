//! A character cursor over the stylesheet text that tracks line and column.

use crate::Position;

pub(super) struct Cursor {
    chars: Vec<char>,
    index: usize,
    position: Position,
}

impl Cursor {
    pub(super) fn new(src: &str) -> Self {
        Self {
            chars: src.chars().collect(),
            index: 0,
            position: Position { line: 1, column: 1 },
        }
    }

    pub(super) fn position(&self) -> Position {
        self.position
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.index >= self.chars.len()
    }

    /// The character `offset` places ahead, without consuming.
    pub(super) fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    pub(super) fn starts_with(&self, text: &str) -> bool {
        text.chars()
            .enumerate()
            .all(|(i, c)| self.peek(i) == Some(c))
    }

    pub(super) fn bump(&mut self) -> Option<char> {
        let c = *self.chars.get(self.index)?;
        self.index += 1;
        if c == '\n' {
            self.position.line += 1;
            self.position.column = 1;
        } else {
            self.position.column += 1;
        }
        Some(c)
    }

    pub(super) fn bump_n(&mut self, n: usize) {
        for _ in 0..n {
            self.bump();
        }
    }

    /// Consume characters while `keep` holds, returning them.
    pub(super) fn take_while(&mut self, keep: impl Fn(char) -> bool) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek(0) {
            if !keep(c) {
                break;
            }
            out.push(c);
            self.bump();
        }
        out
    }
}

pub(super) fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

pub(super) fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || !c.is_ascii()
}

pub(super) fn is_name_char(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit() || c == '-'
}
