//! The component's type name, from its file name.

use std::path::Path;

use crate::CompileError;

/// `counter.wisp` is `Counter`; `text-input.wisp` and `text_input.wisp`
/// are `TextInput`.
///
/// # Errors
/// A file name that is not a Rust type name once the separators are
/// removed.
pub fn component_name(path: &Path) -> Result<String, CompileError> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let name: String = stem.split(['-', '_']).map(capitalise).collect();
    let valid = name.chars().next().is_some_and(char::is_alphabetic)
        && name.chars().all(char::is_alphanumeric);
    let message = if !valid {
        format!(
            "the file name `{stem}` does not make a Rust type name: use letters, digits, `-` or `_`, starting with a letter"
        )
    } else if is_keyword(&name) {
        format!(
            "the file name `{stem}` makes the type name `{name}`, which is a Rust keyword: rename the file"
        )
    } else {
        return Ok(name);
    };
    Err(CompileError {
        path: path.display().to_string(),
        line: 1,
        col: 1,
        message,
    })
}

fn capitalise(part: &str) -> String {
    let mut chars = part.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Rust's strict and reserved keywords (edition 2024), which cannot
/// name a type, module or binding without `r#`.
const KEYWORDS: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Whether `name` is a Rust keyword.
pub(crate) fn is_keyword(name: &str) -> bool {
    KEYWORDS.contains(&name)
}
