//! A page source file's text as `check-docs`' patterns rule reads it:
//! the forms that split or escape UI text in code are undone first.

/// A source file's text, escapes undone, and its path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub path: String,
    pub text: String,
}

impl Source {
    /// `text` from `path`, with what a quote can hide behind undone:
    /// literals joined by `+`, `\'`, `\"`, a Rust line continuation, and
    /// the HTML entities for quotes and `&`.
    #[must_use]
    pub fn new(path: &str, text: &str) -> Self {
        let mut text = join_literals(text)
            .replace("\\'", "'")
            .replace("\\\"", "\"")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&amp;", "&");
        while let Some(at) = text.find("\\\n") {
            let rest = text[at + 2..].trim_start();
            text = format!("{}{rest}", &text[..at]);
        }
        Self {
            path: path.to_owned(),
            text,
        }
    }
}

/// The text with adjacent string literals joined by `+` read as one
/// (`"a " + "b"` reads `"a b"`), as a long message is written in
/// JavaScript.
fn join_literals(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut joined = String::with_capacity(text.len());
    let mut at = 0;
    while at < chars.len() {
        if let Some(after) = join_at(&chars, at) {
            at = after;
        } else {
            joined.push(chars[at]);
            at += 1;
        }
    }
    joined
}

/// Where a join starting at `at` ends: a closing quote, `+` and an
/// opening quote, with any space between.
fn join_at(chars: &[char], at: usize) -> Option<usize> {
    let quote = |c: &char| *c == '"' || *c == '\'';
    if !quote(chars.get(at)?) {
        return None;
    }
    let plus = skip_space(chars, at + 1)?;
    if *chars.get(plus)? != '+' {
        return None;
    }
    let next = skip_space(chars, plus + 1)?;
    // `'+'` is a char literal, not a join: a join has space beside its `+`.
    let spaced = plus > at + 1 || next > plus + 1;
    (spaced && quote(chars.get(next)?)).then_some(next + 1)
}

/// The first index from `from` that is not whitespace.
fn skip_space(chars: &[char], from: usize) -> Option<usize> {
    let mut at = from;
    while chars.get(at)?.is_whitespace() {
        at += 1;
    }
    Some(at)
}

#[cfg(test)]
#[path = "docs_source_text_tests.rs"]
mod tests;
