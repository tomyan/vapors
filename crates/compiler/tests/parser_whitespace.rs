//! One rule for whitespace: a whitespace-only run between two non-text
//! things (an element, a tag boundary or a hole) is dropped when it
//! contains a newline and is one space otherwise; a run containing
//! anything else is kept as written. A hole stays its own part.

mod common;

use common::{element, expr, literal, parse_ok, text_parts};

fn children_of_first(template: &str) -> Vec<(&'static str, String)> {
    text_parts(element(&parse_ok(template).children[0]))
}

#[test]
fn the_newline_between_a_hole_and_a_closing_tag_is_dropped() {
    // When: the reviewer's input
    let parts = children_of_first("<p>\n  hello\n  world {1}\n</p>");

    // Then
    assert_eq!(parts, vec![literal("\n  hello\n  world "), expr("1")]);
}

#[test]
fn a_space_between_two_holes_is_kept_as_one_space() {
    // When
    let parts = children_of_first("<p>{a} {b}</p>");

    // Then
    assert_eq!(parts, vec![expr("a"), literal(" "), expr("b")]);
}

#[test]
fn a_newline_between_two_holes_is_dropped() {
    // When
    let parts = children_of_first("<p>{a}\n{b}</p>");

    // Then
    assert_eq!(parts, vec![expr("a"), expr("b")]);
}

#[test]
fn a_newline_between_an_opening_tag_and_a_hole_is_dropped() {
    // When
    let parts = children_of_first("<p>\n  {a}</p>");

    // Then
    assert_eq!(parts, vec![expr("a")]);
}

#[test]
fn text_beside_an_element_is_kept_as_written() {
    // When
    let doc = parse_ok("<div><b>a</b>\n  text</div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(common::parts(&div.children[1]), vec![literal("\n  text")]);
}

#[test]
fn a_non_breaking_space_between_holes_is_kept() {
    // When: `&nbsp;` decodes to whitespace, but it was written on purpose
    let parts = children_of_first("<p>{a}&nbsp;{b}</p>");

    // Then
    assert_eq!(parts, vec![expr("a"), literal("\u{a0}"), expr("b")]);
}
