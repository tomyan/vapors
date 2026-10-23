//! Translated from sumi's `parser/template/parser_expr_test.go` and
//! `parser_mixed_test.go`: expressions in text, attributes on text
//! elements, and mixed text and inline elements with the JSX newline
//! rule for whitespace gaps. Difference: text holes balance braces
//! here (a Rust expression often contains `{}`); sumi closes a text
//! hole at the first `}`.

mod common;

use common::{attr_literal, element, expr, literal, parse_ok, parts, text_parts};

#[test]
fn parse_text_with_expression_only() {
    // When
    let doc = parse_ok("<span>{count}</span>");

    // Then
    assert_eq!(text_parts(element(&doc.children[0])), [expr("count")]);
}

#[test]
fn parse_text_with_string_and_expression() {
    // When
    let doc = parse_ok("<span>Count: {count}</span>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        [literal("Count: "), expr("count")]
    );
}

#[test]
fn parse_text_with_two_expressions() {
    // When
    let doc = parse_ok("<span>{a} and {b}</span>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        [expr("a"), literal(" and "), expr("b")]
    );
}

#[test]
fn parse_text_with_expression_containing_spaces() {
    // When
    let doc = parse_ok("<span>Count: {count + 1}!</span>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        [literal("Count: "), expr("count + 1"), literal("!")]
    );
}

#[test]
fn parse_text_expression_with_braces_inside() {
    // When
    // `{if` opens a block, so a conditional in a hole is written as a
    // match; the braces inside it balance.
    let doc = parse_ok("<span>{ match x { 1 => \"a\", _ => \"}\" } }</span>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        [expr("match x { 1 => \"a\", _ => \"}\" }")]
    );
}

#[test]
fn parse_text_with_class_attribute() {
    // When
    let doc = parse_ok(r#"<span class="title">Hello</span>"#);

    // Then
    let span = element(&doc.children[0]);
    assert_eq!(attr_literal(span, "class"), "title");
    assert_eq!(text_parts(span), [literal("Hello")]);
}

#[test]
fn parse_text_without_attributes() {
    // When
    let doc = parse_ok("<span>Hello</span>");

    // Then
    assert_eq!(
        element(&doc.children[0]).attributes,
        vapors_compiler::ast::Attributes::default()
    );
}

#[test]
fn parse_text_with_multiple_attributes() {
    // When
    let doc = parse_ok(r#"<span class="title" id="heading">Content</span>"#);

    // Then
    let span = element(&doc.children[0]);
    assert_eq!(attr_literal(span, "class"), "title");
    assert_eq!(attr_literal(span, "id"), "heading");
    assert_eq!(text_parts(span), [literal("Content")]);
}

#[test]
fn parse_text_with_class_inside_box() {
    // When
    let doc = parse_ok(r#"<div><span class="label">Name</span></div>"#);

    // Then
    let span = element(&element(&doc.children[0]).children[0]);
    assert_eq!(attr_literal(span, "class"), "label");
    assert_eq!(text_parts(span), [literal("Name")]);
}

// parser_mixed_test.go

#[test]
fn parse_mixed_text_and_inline_element() {
    // When
    let doc = parse_ok("<p>hello <strong>bold</strong> tail</p>");

    // Then
    let p = element(&doc.children[0]);
    assert_eq!(p.children.len(), 3);
    assert_eq!(parts(&p.children[0]), [literal("hello ")]);
    assert_eq!(element(&p.children[1]).tag, "strong");
    assert_eq!(parts(&p.children[2]), [literal(" tail")]);
}

#[test]
fn parse_mixed_text_with_expressions() {
    // When
    let doc = parse_ok("<p>count: {count} of {total} <em>left</em></p>");

    // Then
    let p = element(&doc.children[0]);
    assert_eq!(p.children.len(), 2);
    let leading = parts(&p.children[0]);
    assert_eq!(leading.len(), 5);
    assert_eq!(leading[1], expr("count"));
    assert_eq!(leading[3], expr("total"));
    assert_eq!(element(&p.children[1]).tag, "em");
}

#[test]
fn parse_newline_gaps_dropped() {
    // When
    let doc = parse_ok("<div>\n\t<div>x</div>\n\t<div>y</div>\n</div>");

    // Then
    assert_eq!(element(&doc.children[0]).children.len(), 2);
}

#[test]
fn parse_single_line_gap_becomes_space() {
    // When
    let doc = parse_ok("<p><strong>a</strong> <em>b</em></p>");

    // Then
    let p = element(&doc.children[0]);
    assert_eq!(p.children.len(), 3);
    assert_eq!(parts(&p.children[1]), [literal(" ")]);
}

#[test]
fn parse_mixed_text_before_control_flow() {
    // When
    let doc = parse_ok("<div>status: {if ok}<span>good</span>{/if}</div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(div.children.len(), 2);
    assert_eq!(parts(&div.children[0]), [literal("status: ")]);
    common::if_block(&div.children[1]);
}

#[test]
fn parse_mixed_text_inside_control_flow_body() {
    // When
    let doc = parse_ok("<div>{if ok}all <strong>good</strong>{/if}</div>");

    // Then
    let block = common::if_block(&element(&doc.children[0]).children[0]);
    assert_eq!(block.then.len(), 2);
    assert_eq!(parts(&block.then[0]), [literal("all ")]);
    assert_eq!(element(&block.then[1]).tag, "strong");
}
