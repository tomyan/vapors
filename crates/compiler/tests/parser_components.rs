//! Translated from sumi's `parser/template/parser_component_test.go`,
//! `component_body_test.go` and the applicable parts of
//! `tagname_test.go`. Differences: a component tag is capitalised here
//! (sumi also accepts any unknown lowercase tag), so `<counter />`
//! becomes `<Counter />`; sumi's `pkg:Name` namespace becomes a Rust
//! path `<pkg::Name />`; the component-tier classification
//! (`ClassifyTag`, `TagRegistryKey`, `TagComponentFile`) has no
//! counterpart and is not translated.

mod common;

use common::{component, element, parse_ok, text};

#[test]
fn parse_component_self_closing() {
    // When
    let doc = parse_ok("<Counter />");

    // Then
    assert_eq!(doc.children.len(), 1);
    assert_eq!(component(&doc.children[0]).name, "Counter");
}

#[test]
fn parse_component_with_attributes() {
    // When
    let doc = parse_ok(r#"<Counter label="Clicks" />"#);

    // Then
    let counter = component(&doc.children[0]);
    assert_eq!(counter.name, "Counter");
    assert_eq!(
        counter.attributes.get("label").unwrap().literal(),
        Some("Clicks")
    );
}

#[test]
fn parse_component_multiple_attributes() {
    // When
    let doc = parse_ok(r#"<Counter label="Clicks" start="0" />"#);

    // Then
    let counter = component(&doc.children[0]);
    assert_eq!(
        counter.attributes.get("label").unwrap().literal(),
        Some("Clicks")
    );
    assert_eq!(
        counter.attributes.get("start").unwrap().literal(),
        Some("0")
    );
}

#[test]
fn parse_component_inside_box() {
    // When
    let doc = parse_ok(r#"<div><Counter label="X" /></div>"#);

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(div.children.len(), 1);
    assert_eq!(component(&div.children[0]).name, "Counter");
}

#[test]
fn parse_multiple_components_in_box() {
    // When
    let doc = parse_ok(r#"<div><Counter label="A" /><Counter label="B" /></div>"#);

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(
        component(&div.children[0])
            .attributes
            .get("label")
            .unwrap()
            .literal(),
        Some("A")
    );
    assert_eq!(
        component(&div.children[1])
            .attributes
            .get("label")
            .unwrap()
            .literal(),
        Some("B")
    );
}

#[test]
fn parse_component_with_closing_tag() {
    // When
    let doc = parse_ok("<Counter></Counter>");

    // Then
    assert_eq!(component(&doc.children[0]).name, "Counter");
}

#[test]
fn parse_component_mixed_with_text() {
    // When
    let doc = parse_ok("<div><span>Title</span><Counter /></div>");

    // Then
    let div = element(&doc.children[0]);
    element(&div.children[0]);
    assert_eq!(component(&div.children[1]).name, "Counter");
}

#[test]
fn parse_path_component_self_closing() {
    // When
    let doc = parse_ok("<widgets::TextInput />");

    // Then
    assert_eq!(component(&doc.children[0]).name, "widgets::TextInput");
}

#[test]
fn parse_path_component_with_closing_tag() {
    // When
    let doc = parse_ok("<widgets::TextInput></widgets::TextInput>");

    // Then
    assert_eq!(component(&doc.children[0]).name, "widgets::TextInput");
}

#[test]
fn parse_path_component_with_attributes() {
    // When
    let doc = parse_ok(r#"<widgets::TextInput bind:value={name} placeholder="Enter name" />"#);

    // Then
    let input = component(&doc.children[0]);
    assert_eq!(
        input.attributes.get("bind:value").unwrap().expr_code(),
        Some("name")
    );
    assert_eq!(
        input.attributes.get("placeholder").unwrap().literal(),
        Some("Enter name")
    );
}

#[test]
fn parse_self_closing_no_space() {
    // When
    let doc = parse_ok("<Counter/>");

    // Then
    assert_eq!(component(&doc.children[0]).name, "Counter");
}

// component_body_test.go

#[test]
fn parse_component_with_text_body() {
    // When
    let doc = parse_ok("<Card>Hello</Card>");

    // Then
    let card = component(&doc.children[0]);
    assert_eq!(card.children.len(), 1);
    text(&card.children[0]);
}

#[test]
fn parse_component_with_snippet_and_body() {
    // When
    let doc = parse_ok("<Card>{snippet footer()}<span>F</span>{/snippet}<span>Body</span></Card>");

    // Then
    let card = component(&doc.children[0]);
    assert_eq!(card.children.len(), 2);
    assert!(
        matches!(&card.children[0], vapors_compiler::ast::Node::Snippet(s) if s.name == "footer")
    );
    element(&card.children[1]);
}

#[test]
fn parse_self_closing_component_has_no_children() {
    // When
    let doc = parse_ok("<Card />");

    // Then
    assert_eq!(component(&doc.children[0]).children, Vec::new());
}
