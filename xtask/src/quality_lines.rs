//! Lines of code in a function body, counted as clippy's `too_many_lines`
//! counts them (`clippy_lints/src/functions/too_many_lines.rs`): the text
//! between the body's braces, trimmed; a line counts when it holds
//! something besides whitespace and comments. The hard check is
//! clippy's; this count is for the band below it, so the two must agree,
//! and `docs/decisions/2026-11-13-quality-thresholds.md` records that
//! they did on every function of main.

use proc_macro2::LineColumn;

/// Byte offsets of each line's start, for turning a span into text.
pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Index `text`.
    pub fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Self { text, starts }
    }

    /// The text from `start` to `end`, both as proc-macro2 reports them:
    /// 1-based lines, 0-based columns in characters.
    pub fn slice(&self, start: LineColumn, end: LineColumn) -> &'a str {
        &self.text[self.offset(start)..self.offset(end)]
    }

    fn offset(&self, at: LineColumn) -> usize {
        let line_start = self
            .starts
            .get(at.line - 1)
            .copied()
            .unwrap_or(self.text.len());
        let line_end = self.starts.get(at.line).copied().unwrap_or(self.text.len());
        let line = &self.text[line_start..line_end];
        let in_line = line
            .char_indices()
            .nth(at.column)
            .map_or(line.len(), |(i, _)| i);
        line_start + in_line
    }
}

/// Lines with code in a body's text, braces included.
pub fn code_lines(body: &str) -> usize {
    let inner = body
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .unwrap_or(body);
    let mut in_comment = false;
    inner
        .trim()
        .lines()
        .filter(|line| line_has_code(line, &mut in_comment))
        .count()
}

/// Whether `line` holds code outside comments; `in_comment` carries a
/// block comment across lines. A `//` inside a string literal ends the
/// line's code, as it does for clippy.
fn line_has_code(mut line: &str, in_comment: &mut bool) -> bool {
    let mut code = false;
    loop {
        line = line.trim_start();
        if line.is_empty() {
            return code;
        }
        if *in_comment {
            let Some(end) = line.find("*/") else {
                return code;
            };
            line = &line[end + 2..];
            *in_comment = false;
            continue;
        }
        let multi = line.find("/*").unwrap_or(line.len());
        let single = line.find("//").unwrap_or(line.len());
        code |= multi > 0 && single > 0;
        if multi < single {
            line = &line[multi + 2..];
            *in_comment = true;
            continue;
        }
        return code;
    }
}

#[cfg(test)]
#[path = "quality_lines_tests.rs"]
mod tests;
