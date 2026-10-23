//! Helpers shared by the codegen tests.

#![allow(dead_code)] // each test binary uses a subset

use vapors_compiler::{CompileError, compile_str};

/// Compiles a whole `.wisp` text named `counter.wisp`.
pub fn generated(text: &str) -> String {
    compile_str("counter.wisp", text).unwrap_or_else(|err| panic!("{err}"))
}

/// Compiles a template with the given script.
pub fn generated_with(script: &str, template: &str) -> String {
    generated(&format!("<script>\n{script}\n</script>\n{template}"))
}

pub fn refusal(text: &str) -> CompileError {
    match compile_str("counter.wisp", text) {
        Ok(code) => panic!("expected a refusal, generated:\n{code}"),
        Err(err) => err,
    }
}

/// The code with every whitespace character and every trailing comma
/// removed: prettyplease wraps long calls and adds a comma after the
/// last argument, and the tests pin tokens, not layout.
pub fn squashed(code: &str) -> String {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    compact
        .replace(",)", ")")
        .replace(",}", "}")
        .replace(",]", "]")
}

/// A needle squashed, without a comma at its end (the code's last
/// field or argument has none once squashed).
fn squashed_needle(needle: &str) -> String {
    squashed(needle).trim_end_matches(',').to_string()
}

/// Position of `needle` in `haystack`, both squashed, panicking with the text.
pub fn at(haystack: &str, needle: &str) -> usize {
    squashed(haystack)
        .find(&squashed_needle(needle))
        .unwrap_or_else(|| panic!("missing {needle:?} in:\n{haystack}"))
}

pub fn assert_has(code: &str, needle: &str) {
    assert!(
        squashed(code).contains(&squashed_needle(needle)),
        "missing {needle:?} in:\n{code}"
    );
}

pub fn assert_lacks(code: &str, needle: &str) {
    assert!(
        !squashed(code).contains(&squashed_needle(needle)),
        "unexpected {needle:?} in:\n{code}"
    );
}
