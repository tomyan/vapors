//! Character references in text and quoted attribute values: the named
//! ones HTML text needs and numeric ones are decoded; an unknown or
//! invalid one is refused at its position; an `&` that does not start
//! a reference is kept as written.

mod common;

use common::{attr_literal, element, literal, parse_err, parse_ok, text_parts};

fn paragraph_text(template: &str) -> Vec<(&'static str, String)> {
    text_parts(element(&parse_ok(template).children[0]))
}

#[test]
fn named_references_are_decoded() {
    // When
    let parts = paragraph_text("<p>a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos;</p>");

    // Then
    assert_eq!(parts, vec![literal("a & b <c> \"d\" 'e'")]);
}

#[test]
fn a_non_breaking_space_is_decoded() {
    // When
    let parts = paragraph_text("<p>1&nbsp;kg</p>");

    // Then
    assert_eq!(parts, vec![literal("1\u{a0}kg")]);
}

#[test]
fn decimal_and_hex_references_are_decoded() {
    // When
    let parts = paragraph_text("<p>&#65;&#x42;&#X43;</p>");

    // Then
    assert_eq!(parts, vec![literal("ABC")]);
}

#[test]
fn an_ampersand_that_starts_no_reference_is_kept() {
    // When
    let parts = paragraph_text("<p>salt & pepper &amp</p>");

    // Then
    assert_eq!(parts, vec![literal("salt & pepper &amp")]);
}

#[test]
fn an_unknown_named_reference_is_refused_at_its_position() {
    // When
    let err = parse_err("<p>\n  x &bogus; y\n</p>");

    // Then
    assert_eq!((err.line, err.col), (2, 5), "{err}");
    assert!(
        err.message.contains("&bogus;") && err.message.contains("&amp;"),
        "{err}"
    );
}

#[test]
fn a_numeric_reference_to_no_character_is_refused() {
    // When
    let err = parse_err("<p>ok &#x110000;</p>");

    // Then
    assert_eq!((err.line, err.col), (1, 7), "{err}");
    assert!(err.message.contains("&#x110000;"), "{err}");
}

#[test]
fn references_in_a_quoted_attribute_are_decoded() {
    // When
    let doc = parse_ok("<a title=\"fish &amp; chips\"></a>");

    // Then
    assert_eq!(
        attr_literal(element(&doc.children[0]), "title"),
        "fish & chips"
    );
}

#[test]
fn an_unknown_reference_in_an_attribute_is_refused_at_its_position() {
    // When
    let err = parse_err("<a title=\"x &nope; y\"></a>");

    // Then
    assert_eq!((err.line, err.col), (1, 13), "{err}");
}
