//! The one error type: a position in a file and a message that names
//! the fix.

use std::fmt;

/// A refusal, displayed as `file:line:col: message` so editors jump to
/// it. Lines and columns are 1-based; columns count characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    /// The file as it was named to the compiler.
    pub path: String,
    /// 1-based line.
    pub line: usize,
    /// 1-based column in characters.
    pub col: usize,
    /// What was refused and what to do instead.
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.path, self.line, self.col, self.message
        )
    }
}

impl std::error::Error for CompileError {}
