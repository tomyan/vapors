//! Translated from sumi's `parser/template/snippet_test.go`. Snippets
//! and `{render}` parse here as there; code generation for them is
//! deferred (docs/components.md says so), so a component using them
//! is refused at that stage, not here.

mod common;

use common::{element, parse_ok};
use vapors_compiler::ast::Node;

#[test]
fn parse_snippet_definition() {
    // When
    let doc = parse_ok("<div>{snippet item(name: &str)}<span>{name}</span>{/snippet}</div>");

    // Then
    let div = element(&doc.children[0]);
    let Node::Snippet(snippet) = &div.children[0] else {
        panic!("expected a snippet, got {:?}", div.children[0]);
    };
    assert_eq!(snippet.name, "item");
    assert_eq!(snippet.params, "(name: &str)");
    assert_eq!(snippet.children.len(), 1);
}

#[test]
fn parse_render_snippet() {
    // When
    let doc = parse_ok(r#"<div>{render item("hello")}</div>"#);

    // Then
    let div = element(&doc.children[0]);
    let Node::Render(render) = &div.children[0] else {
        panic!("expected a render, got {:?}", div.children[0]);
    };
    assert_eq!(render.name, "item");
    assert_eq!(render.args, r#""hello""#);
}

#[test]
fn unclosed_snippet_is_refused_by_name() {
    // When
    let err = common::parse_err("<div>{snippet item()}<span>x</span></div>");

    // Then
    assert!(
        err.message.contains("{/snippet}") && err.message.contains("item"),
        "{err}"
    );
}
