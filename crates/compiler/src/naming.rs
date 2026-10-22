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
    if valid {
        return Ok(name);
    }
    Err(CompileError {
        path: path.display().to_string(),
        line: 1,
        col: 1,
        message: format!(
            "the file name `{stem}` does not make a Rust type name: use letters, digits, `-` or `_`, starting with a letter"
        ),
    })
}

fn capitalise(part: &str) -> String {
    let mut chars = part.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
