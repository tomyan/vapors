//! The one error type: every refusal names the line and column of the
//! stylesheet text it refuses, and says what would be accepted instead.

use std::fmt;

/// A position in the stylesheet text, both 1-based.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Position {
    /// Line, starting at 1.
    pub line: u32,
    /// Column in characters, starting at 1.
    pub column: u32,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// A refusal: where in the stylesheet, and what was expected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// Where the refused text starts.
    pub position: Position,
    /// What was found and what would be accepted.
    pub message: String,
}

impl Error {
    /// A refusal at `position`.
    #[must_use]
    pub fn new(position: Position, message: impl Into<String>) -> Self {
        Self {
            position,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.position, self.message)
    }
}

impl std::error::Error for Error {}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
