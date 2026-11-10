//! Reading SVG path data's numbers, arc flags and separators, with the
//! byte offset of anything not understood.

use std::fmt;

/// Why path data was refused: the byte offset of the first character
/// not understood, and what was expected there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathError {
    /// Byte offset into the path data.
    pub offset: usize,
    /// What was wrong, and the fix.
    pub reason: String,
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "path data at byte {}: {}", self.offset, self.reason)
    }
}

impl std::error::Error for PathError {}

/// A position in path data.
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(data: &'a str) -> Self {
        Self {
            bytes: data.as_bytes(),
            at: 0,
        }
    }

    pub(crate) fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    pub(crate) fn advance(&mut self) {
        self.at += 1;
    }

    pub(crate) fn at_end(&self) -> bool {
        self.at >= self.bytes.len()
    }

    /// A refusal at the current position.
    pub(crate) fn error(&self, reason: impl Into<String>) -> PathError {
        PathError {
            offset: self.at,
            reason: reason.into(),
        }
    }

    pub(crate) fn skip_space(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.advance();
        }
    }

    /// Skips whitespace with at most one comma in it.
    pub(crate) fn skip_separator(&mut self) {
        self.skip_space();
        if self.peek() == Some(b',') {
            self.advance();
            self.skip_space();
        }
    }

    /// Whether a number starts here, after any separator.
    pub(crate) fn number_follows(&mut self) -> bool {
        self.skip_separator();
        self.peek()
            .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.'))
    }

    /// An arc flag, `0` or `1`, after any separator.
    pub(crate) fn flag(&mut self) -> Result<bool, PathError> {
        self.skip_separator();
        let flag = match self.peek() {
            Some(b'0') => false,
            Some(b'1') => true,
            _ => return Err(self.error("expected an arc flag, 0 or 1")),
        };
        self.advance();
        Ok(flag)
    }

    /// A number in SVG's syntax, after any separator: a sign, digits
    /// with an optional fraction (or a fraction alone), and an optional
    /// exponent.
    pub(crate) fn number(&mut self) -> Result<f32, PathError> {
        self.skip_separator();
        let start = self.at;
        self.skip_sign();
        let mantissa = self.digits() + self.fraction();
        if mantissa == 0 {
            self.at = start;
            return Err(self.error("expected a number"));
        }
        self.exponent()?;
        let text = std::str::from_utf8(&self.bytes[start..self.at]).expect("ASCII digits");
        text.parse()
            .map_err(|_| self.error(format!("{text:?} is not a number")))
    }

    fn skip_sign(&mut self) {
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.advance();
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.advance();
        }
        self.at - start
    }

    fn fraction(&mut self) -> usize {
        if self.peek() != Some(b'.') {
            return 0;
        }
        self.advance();
        self.digits()
    }

    fn exponent(&mut self) -> Result<(), PathError> {
        if !matches!(self.peek(), Some(b'e' | b'E')) {
            return Ok(());
        }
        self.advance();
        self.skip_sign();
        if self.digits() == 0 {
            return Err(self.error("expected the exponent's digits"));
        }
        Ok(())
    }
}

/// SVG's whitespace.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}
