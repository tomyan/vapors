//! Helpers shared by the parser tests: parse a template alone, and
//! pull nodes apart without repeating the match arms.

#![allow(dead_code)] // each test binary uses a subset

use vapors_compiler::ast::{
    AttrValue, Component, ComponentTag, Document, Element, ForBlock, IfBlock, Node, Part, Text,
};
use vapors_compiler::{CompileError, parse_str};

/// Parses `template` as a whole `.wisp` file named `test.wisp`.
pub fn parse(template: &str) -> Result<Document, CompileError> {
    parse_str("test.wisp", template).map(|component| component.template)
}

/// Parses a whole file.
pub fn parse_file(text: &str) -> Result<Component, CompileError> {
    parse_str("test.wisp", text)
}

pub fn parse_ok(template: &str) -> Document {
    parse(template).unwrap_or_else(|err| panic!("{err}"))
}

pub fn parse_err(template: &str) -> CompileError {
    match parse(template) {
        Ok(doc) => panic!("expected a refusal, parsed {doc:?}"),
        Err(err) => err,
    }
}

pub fn element(node: &Node) -> &Element {
    match node {
        Node::Element(element) => element,
        other => panic!("expected an element, got {other:?}"),
    }
}

pub fn component(node: &Node) -> &ComponentTag {
    match node {
        Node::Component(component) => component,
        other => panic!("expected a component, got {other:?}"),
    }
}

pub fn if_block(node: &Node) -> &IfBlock {
    match node {
        Node::If(block) => block,
        other => panic!("expected an if block, got {other:?}"),
    }
}

pub fn for_block(node: &Node) -> &ForBlock {
    match node {
        Node::For(block) => block,
        other => panic!("expected a for block, got {other:?}"),
    }
}

pub fn text(node: &Node) -> &Text {
    match node {
        Node::Text(text) => text,
        other => panic!("expected text, got {other:?}"),
    }
}

/// The parts of a text node as `("literal", "...")` / `("expr", "...")`.
pub fn parts(node: &Node) -> Vec<(&'static str, String)> {
    text(node)
        .parts
        .iter()
        .map(|part| match part {
            Part::Literal(value) => ("literal", value.clone()),
            Part::Expr(expr) => ("expr", expr.code.clone()),
        })
        .collect()
}

pub fn literal(value: &str) -> (&'static str, String) {
    ("literal", value.to_string())
}

pub fn expr(code: &str) -> (&'static str, String) {
    ("expr", code.to_string())
}

/// A literal attribute's value.
pub fn attr_literal(element: &Element, name: &str) -> String {
    match element.attributes.get(name) {
        Some(AttrValue::Literal(value)) => value.clone(),
        other => panic!("expected literal attribute {name}, got {other:?}"),
    }
}

/// An expression attribute's code.
pub fn attr_expr(element: &Element, name: &str) -> String {
    match element.attributes.get(name) {
        Some(AttrValue::Expr(expr)) => expr.code.clone(),
        other => panic!("expected expression attribute {name}, got {other:?}"),
    }
}

/// The single text child of an element, as parts.
pub fn text_parts(element: &Element) -> Vec<(&'static str, String)> {
    assert_eq!(element.children.len(), 1, "one text child: {element:?}");
    parts(&element.children[0])
}
