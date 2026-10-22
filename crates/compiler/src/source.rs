//! A file's text with byte offsets resolved to lines and columns.

use crate::CompileError;

pub(crate) struct Source {
    pub path: String,
    pub text: String,
}

impl Source {
    pub fn new(path: &str, text: &str) -> Source {
        Source {
            path: path.to_string(),
            text: text.to_string(),
        }
    }

    /// 1-based line and character column of a byte offset.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let before = &self.text[..offset];
        let line = before.matches('\n').count() + 1;
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let col = before[line_start..].chars().count() + 1;
        (line, col)
    }

    /// `line:col` of an offset, for messages that point elsewhere.
    pub fn describe(&self, offset: usize) -> String {
        let (line, col) = self.position(offset);
        format!("{line}:{col}")
    }

    pub fn error(&self, offset: usize, message: impl Into<String>) -> CompileError {
        let (line, col) = self.position(offset);
        CompileError {
            path: self.path.clone(),
            line,
            col,
            message: message.into(),
        }
    }
}
