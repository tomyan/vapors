//! Rule refusals (`src/parse/rules.rs`): unterminated blocks and comments, stray braces, missing and empty selectors. Translated from sumi `parser/style/style_edge_test.go`.

mod support;

use support::refusal;

#[test]
fn unterminated_block_is_refused_at_the_open_brace() {
    // When
    let err = refusal(".title { color: green;");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 8));
    assert!(err.message.contains('}'), "{err}");
}

#[test]
fn missing_selector_before_brace_is_refused() {
    // When
    let err = refusal("{ color: green; }");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 1));
    assert!(err.message.contains("selector"), "{err}");
}

#[test]
fn unterminated_comment_is_refused() {
    // When
    let err = refusal("/* unterminated comment .title { color: green; }");

    // Then
    assert_eq!((err.position.line, err.position.column), (1, 1));
    assert!(err.message.contains("comment"), "{err}");
}

#[test]
fn stray_close_brace_is_refused() {
    // When
    let err = refusal(".x { color: red; } }");

    // Then
    assert_eq!(err.position.column, 20);
}

#[test]
fn empty_selector_in_list_is_refused() {
    let err = refusal("a, { color: red; }");
    assert!(err.message.contains("empty selector"), "{err}");
}
