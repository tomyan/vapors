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
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(data: &'a str) -> Self {
        Self {
            text: data,
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

    /// A refusal at the current position. Every refusal points at the
    /// doc here, so none can leave it out.
    pub(crate) fn error(&self, reason: impl Into<String>) -> PathError {
        PathError {
            offset: self.at,
            reason: format!("{} (docs/icons.md lists the subset)", reason.into()),
        }
    }

    /// The character at the cursor, quoted, for a message. `at` only ever
    /// advances over ASCII, so it is on a character boundary.
    pub(crate) fn found(&self) -> String {
        match self.text[self.at..].chars().next() {
            Some(found) => format!("{found:?}"),
            None => "the end of the data".to_string(),
        }
    }

    /// Only whitespace may follow a command letter (SVG 2's grammar;
    /// Chromium refuses `M,1 1`).
    pub(crate) fn refuse_comma_after_command(&mut self) -> Result<(), PathError> {
        self.skip_space();
        if self.peek() == Some(b',') {
            return Err(self.error(
                "a comma cannot follow a command letter; remove it (write M1 1, not M,1 1)",
            ));
        }
        Ok(())
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
            _ => return Err(self.expected("an arc flag, 0 or 1", "write each arc flag as 0 or 1")),
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
        let mantissa = self.digits() + self.fraction()?;
        if mantissa == 0 {
            self.at = start;
            return Err(self.expected("a number", "write the argument as digits such as 4 or -1.5"));
        }
        self.exponent()?;
        let text = &self.text[start..self.at];
        let value: f32 = text.parse().unwrap_or(f32::NAN);
        if value.is_finite() {
            // -0 draws as 0, so it is 0: one canonical form for one shape.
            return Ok(if value == 0.0 { 0.0 } else { value });
        }
        self.at = start;
        Err(self.error(format!(
            "{text:?} does not fit a 32-bit float; write a number under 3.4e38"
        )))
    }

    /// A refusal at the current position: `what` was expected, what is
    /// here was found, and `fix` says what to write.
    fn expected(&self, what: &str, fix: &str) -> PathError {
        self.error(format!("expected {what}, found {}; {fix}", self.found()))
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

    /// The digits after a decimal point; at least one must follow the
    /// point, as Chromium reads it (`1.` is refused).
    fn fraction(&mut self) -> Result<usize, PathError> {
        if self.peek() != Some(b'.') {
            return Ok(0);
        }
        self.advance();
        let digits = self.digits();
        if digits == 0 {
            return Err(self.error(
                "expected a digit after the decimal point to make a number such as 1.5; Chromium refuses 1.",
            ));
        }
        Ok(digits)
    }

    fn exponent(&mut self) -> Result<(), PathError> {
        if !matches!(self.peek(), Some(b'e' | b'E')) {
            return Ok(());
        }
        self.advance();
        self.skip_sign();
        if self.digits() == 0 {
            return Err(
                self.error("expected the exponent's digits after e; write 1e2, or drop the e")
            );
        }
        Ok(())
    }
}

/// SVG's whitespace.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}
