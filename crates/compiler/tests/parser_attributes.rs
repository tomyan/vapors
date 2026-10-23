//! Translated from sumi's `parser/template/parser_attr_test.go`. sumi
//! stores an expression attribute as the string `"{expr}"`; here it is
//! `AttrValue::Expr` holding the code without braces.

mod common;

use common::{attr_expr, attr_literal, component, element, parse_err, parse_ok};

#[test]
fn parse_expression_attribute() {
    // When
    let doc = parse_ok("<div onkeydown={handle_key}><span>hello</span></div>");

    // Then
    assert_eq!(
        attr_expr(element(&doc.children[0]), "onkeydown"),
        "handle_key"
    );
}

#[test]
fn parse_expression_attribute_with_spaces() {
    // When
    let doc = parse_ok("<div title={w - 2}><span>hello</span></div>");

    // Then
    assert_eq!(attr_expr(element(&doc.children[0]), "title"), "w - 2");
}

#[test]
fn parse_expression_attribute_balances_braces() {
    // When
    let doc = parse_ok("<div class={format!(\"{}\", Point { x: 1 }.x)}></div>");

    // Then
    assert_eq!(
        attr_expr(element(&doc.children[0]), "class"),
        "format!(\"{}\", Point { x: 1 }.x)"
    );
}

#[test]
fn parse_shorthand_attribute() {
    // When
    let doc = parse_ok("<Counter {count} />");

    // Then
    let counter = component(&doc.children[0]);
    assert_eq!(counter.attributes.len(), 1);
    assert_eq!(counter.attributes[0].name, "count");
    assert!(matches!(
        &counter.attributes[0].value,
        vapors_compiler::ast::AttrValue::Expr(expr) if expr.code == "count"
    ));
}

#[test]
fn parse_mixed_attribute_syntax() {
    // When
    let doc = parse_ok(r#"<Counter label="Clicks" count={my_count} {on_reset} />"#);

    // Then
    let counter = component(&doc.children[0]);
    let names: Vec<&str> = counter.attributes.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["label", "count", "on_reset"]);
}

#[test]
fn parse_box_expression_attribute() {
    // When
    let doc = parse_ok("<div data-cursor-x={cursor}><span>hi</span></div>");

    // Then
    assert_eq!(
        attr_expr(element(&doc.children[0]), "data-cursor-x"),
        "cursor"
    );
}

#[test]
fn parse_quoted_attribute_unchanged() {
    // When
    let doc = parse_ok(r#"<div class="row"><span>hi</span></div>"#);

    // Then
    assert_eq!(attr_literal(element(&doc.children[0]), "class"), "row");
}

#[test]
fn parse_bind_and_event_attribute_names() {
    // When
    let doc = parse_ok(r#"<input bind:value={name} oninput={changed} aria-label="Name" />"#);

    // Then
    let input = element(&doc.children[0]);
    assert_eq!(attr_expr(input, "bind:value"), "name");
    assert_eq!(attr_expr(input, "oninput"), "changed");
    assert_eq!(attr_literal(input, "aria-label"), "Name");
}

// Refusals (vapors additions: sumi reports these without a position).

#[test]
fn bare_attribute_without_a_value_is_refused_at_its_position() {
    // When
    let err = parse_err("<div>\n  <input disabled />\n</div>");

    // Then
    assert_eq!((err.line, err.col), (2, 10), "{err}");
    assert!(err.message.contains("disabled"), "{err}");
    assert!(err.message.contains('='), "names the fix: {err}");
}

#[test]
fn single_quoted_attribute_is_refused() {
    // When
    let err = parse_err("<div class='row'></div>");

    // Then
    assert_eq!((err.line, err.col), (1, 12), "{err}");
    assert!(err.message.contains('"'), "{err}");
}

#[test]
fn unterminated_attribute_expression_is_refused() {
    // When
    let err = parse_err("<div class={cls(x)></div>");

    // Then
    assert_eq!((err.line, err.col), (1, 12), "{err}");
    assert!(err.message.contains("class"), "{err}");
}

#[test]
fn duplicate_attribute_is_refused() {
    // When
    let err = parse_err(r#"<div class="a" class="b"></div>"#);

    // Then
    assert_eq!((err.line, err.col), (1, 16), "{err}");
    assert!(err.message.contains("class"), "{err}");
}
