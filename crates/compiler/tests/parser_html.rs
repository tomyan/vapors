//! Translated from sumi's `parser/template/parser_html_test.go`: the
//! HTML element vocabulary, self-closing and void tags, and the refusal
//! of the removed `<box>` and `<text>` tags.

mod common;

use common::{element, parse_err, parse_ok, parts, text_parts};

#[test]
fn parse_html_text_like_element() {
    // When
    let doc = parse_ok(r#"<h1 class="title">Hello {name}</h1>"#);

    // Then
    let h1 = element(&doc.children[0]);
    assert_eq!(h1.tag, "h1");
    assert_eq!(common::attr_literal(h1, "class"), "title");
    assert_eq!(text_parts(h1).len(), 2);
}

#[test]
fn parse_html_container_element() {
    // When
    let doc = parse_ok(r#"<div class="panel"><span>inside</span></div>"#);

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(element(&div.children[0]).tag, "span");
}

#[test]
fn parse_html_self_closing() {
    // When
    let doc = parse_ok("<div><hr/><span>after</span></div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(element(&div.children[0]).tag, "hr");
    assert_eq!(element(&div.children[1]).tag, "span");
}

#[test]
fn parse_html_void_element_without_slash() {
    // vapors difference: HTML void elements close themselves, as in HTML.
    // When
    let doc = parse_ok(r#"<div><input type="text"><br><span>after</span></div>"#);

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(div.children.len(), 3);
    assert_eq!(element(&div.children[2]).tag, "span");
}

#[test]
fn parse_html_control_flow_body() {
    // When
    let doc = parse_ok("<ul>{for item in items.get()}<li>{item}</li>{/for}</ul>");

    // Then
    let ul = element(&doc.children[0]);
    assert_eq!(ul.tag, "ul");
    common::for_block(&ul.children[0]);
}

#[test]
fn uppercase_tag_is_a_component() {
    // When
    let doc = parse_ok(r#"<Counter label="x" />"#);

    // Then
    assert_eq!(common::component(&doc.children[0]).name, "Counter");
}

#[test]
fn legacy_box_and_text_tags_are_refused_with_a_pointer() {
    // When / Then
    let err = parse_err("<box></box>");
    assert!(err.message.contains("div"), "{err}");
    let err = parse_err("<span>ok</span><text>no</text>");
    assert!(err.message.contains("span"), "{err}");
}

#[test]
fn parse_button_label_with_expression() {
    // sumi keeps `button` a container with an implicit text child; here
    // every element holds its text as a child, so the shape is the same.
    // When
    let doc = parse_ok("<button onclick={save}>Save {n}</button>");

    // Then
    let button = element(&doc.children[0]);
    assert_eq!(button.tag, "button");
    assert_eq!(common::attr_expr(button, "onclick"), "save");
    assert_eq!(parts(&button.children[0]).len(), 2);
}
