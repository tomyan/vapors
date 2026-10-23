//! Translated from sumi's `parser/template/parser_test.go` and
//! `parser_html_test.go` (the latter in `parser_html.rs`). Where sumi splits elements into
//! `TextElement` and `BoxElement`, vapors has one `Element` whose
//! text body is a `Text` child, so "is a `TextElement` with parts"
//! becomes "has one text child with parts". `<box>` and `<text>` are
//! refused here as there, with a pointer to `div` and `span`.

mod common;

use common::{element, literal, parse_err, parse_ok, text_parts};

#[test]
fn parse_single_text_element() {
    // When
    let doc = parse_ok("<span>Hello</span>");

    // Then
    assert_eq!(doc.children.len(), 1);
    let span = element(&doc.children[0]);
    assert_eq!(span.tag, "span");
    assert_eq!(text_parts(span), [literal("Hello")]);
}

#[test]
fn parse_two_text_elements() {
    // When
    let doc = parse_ok("<span>Hello</span><span>World</span>");

    // Then
    assert_eq!(doc.children.len(), 2);
    assert_eq!(text_parts(element(&doc.children[0])), [literal("Hello")]);
    assert_eq!(text_parts(element(&doc.children[1])), [literal("World")]);
}

#[test]
fn parse_empty_text_element() {
    // When
    let doc = parse_ok("<span></span>");

    // Then
    assert_eq!(element(&doc.children[0]).children, Vec::new());
}

#[test]
fn parse_whitespace_between_elements_ignored() {
    // When
    let doc = parse_ok("  <span>A</span>  \n  <span>B</span>  ");

    // Then
    assert_eq!(doc.children.len(), 2);
    assert_eq!(text_parts(element(&doc.children[0])), [literal("A")]);
    assert_eq!(text_parts(element(&doc.children[1])), [literal("B")]);
}

#[test]
fn parse_missing_closing_tag_is_refused() {
    // When
    let err = parse_err("<span>Hello");

    // Then
    assert!(err.message.contains("</span>"), "{err}");
}

#[test]
fn parse_box_with_single_text_child() {
    // When
    let doc = parse_ok("<div><span>Hello</span></div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(div.tag, "div");
    assert_eq!(div.children.len(), 1);
    assert_eq!(text_parts(element(&div.children[0])), [literal("Hello")]);
}

#[test]
fn parse_box_with_attribute_and_two_children() {
    // When
    let doc = parse_ok(r#"<div class="column"><span>A</span><span>B</span></div>"#);

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(common::attr_literal(div, "class"), "column");
    assert_eq!(div.children.len(), 2);
}

#[test]
fn parse_box_with_multiple_attributes() {
    // When
    let doc = parse_ok(r#"<div id="a" class="b" style="c" data-pad="1 2"><span>X</span></div>"#);

    // Then
    let div = element(&doc.children[0]);
    for (name, want) in [
        ("id", "a"),
        ("class", "b"),
        ("style", "c"),
        ("data-pad", "1 2"),
    ] {
        assert_eq!(common::attr_literal(div, name), want);
    }
}

#[test]
fn parse_nested_boxes() {
    // When
    let doc = parse_ok("<div><div><span>Deep</span></div></div>");

    // Then
    let outer = element(&doc.children[0]);
    let inner = element(&outer.children[0]);
    assert_eq!(inner.tag, "div");
    assert_eq!(text_parts(element(&inner.children[0])), [literal("Deep")]);
}

#[test]
fn parse_empty_box() {
    // When
    let doc = parse_ok("<div></div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(div.children, Vec::new());
    assert_eq!(div.attributes, vapors_compiler::ast::Attributes::default());
}

#[test]
fn parse_box_missing_closing_tag_is_refused() {
    // When
    let err = parse_err("<div><span>Hello</span>");

    // Then
    assert!(err.message.contains("</div>"), "{err}");
}

#[test]
fn parse_box_with_whitespace_between_children() {
    // When
    let doc = parse_ok("<div>\n  <span>A</span>\n  <span>B</span>\n</div>");

    // Then
    assert_eq!(element(&doc.children[0]).children.len(), 2);
}

#[test]
fn parse_whitespace_inside_text_preserved() {
    // When
    let doc = parse_ok("<span>  Hello  </span>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        [literal("  Hello  ")]
    );
}
