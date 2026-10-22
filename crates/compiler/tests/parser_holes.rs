//! What a `{...}` hole may hold: raw strings, whose quotes and braces
//! do not end the hole, and the refusal of a Rust if-expression, which
//! `{if` would otherwise read as a template block.

mod common;

use common::{element, expr, parse_err, parse_ok, text_parts};

#[test]
fn a_raw_string_with_a_brace_and_a_quote_stays_in_its_hole() {
    // When
    let doc = parse_ok("<p>{r#\"a \"}\" b\"#}</p>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        vec![expr("r#\"a \"}\" b\"#")]
    );
}

#[test]
fn a_raw_string_ending_in_a_backslash_stays_in_its_hole() {
    // When: in a raw string `\` does not escape the closing quote
    let doc = parse_ok("<p>{r\"\\\"}</p>");

    // Then
    assert_eq!(text_parts(element(&doc.children[0])), vec![expr("r\"\\\"")]);
}

#[test]
fn a_byte_raw_string_stays_in_its_hole() {
    // When
    let doc = parse_ok("<p>{br##\"}\"##.len()}</p>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        vec![expr("br##\"}\"##.len()")]
    );
}

#[test]
fn a_c_raw_string_stays_in_its_hole() {
    // When
    let doc = parse_ok("<p>{cr#\"a\"}\"#.count_bytes()}</p>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        vec![expr("cr#\"a\"}\"#.count_bytes()")]
    );
}

#[test]
fn a_rust_if_expression_in_a_hole_is_refused_with_the_alternatives() {
    // When
    let err = parse_err("<p>\n  {if done { \"yes\" } else { \"no\" }}\n</p>");

    // Then
    assert_eq!((err.line, err.col), (2, 3), "{err}");
    assert!(
        err.message.contains("Memo") && err.message.contains("<script>"),
        "{err}"
    );
}
