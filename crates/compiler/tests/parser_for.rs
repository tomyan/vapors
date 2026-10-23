//! Translated from the `{for}` half of sumi's
//! `parser/template/parser_controlflow_test.go`. sumi's loop clause is
//! a Go range clause kept verbatim; here it is
//! `{for PATTERN in EXPR key=EXPR}`, so `Clause` becomes `pattern` and
//! `iterable`.

mod common;

use common::{element, for_block, if_block, parse_err, parse_ok};

#[test]
fn parse_for_basic() {
    // When
    let doc = parse_ok("{for i in items}<span>{i}</span>{/for}");

    // Then
    let block = for_block(&doc.children[0]);
    assert_eq!(block.pattern, "i");
    assert_eq!(block.iterable.code, "items");
    assert_eq!(block.children.len(), 1);
}

#[test]
fn parse_for_with_index() {
    // When
    let doc = parse_ok("{for (i, item) in items.iter().enumerate()}<span>{i}: {item}</span>{/for}");

    // Then
    let block = for_block(&doc.children[0]);
    assert_eq!(block.pattern, "(i, item)");
    assert_eq!(block.iterable.code, "items.iter().enumerate()");
}

#[test]
fn parse_for_in_box() {
    // When
    let doc = parse_ok("<div>{for i in items}<span>{i}</span>{/for}</div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(for_block(&div.children[0]).iterable.code, "items");
}

#[test]
fn parse_for_containing_if() {
    // When
    let doc = parse_ok("{for i in items}{if i > 0}<span>sep</span>{/if}<span>{i}</span>{/for}");

    // Then
    let block = for_block(&doc.children[0]);
    assert_eq!(block.children.len(), 2);
    if_block(&block.children[0]);
    element(&block.children[1]);
}

#[test]
fn parse_for_with_key() {
    // When
    let doc = parse_ok("{for (i, item) in items key=item.id}<span>{item}</span>{/for}");

    // Then
    let block = for_block(&doc.children[0]);
    assert_eq!(block.pattern, "(i, item)");
    assert_eq!(block.iterable.code, "items");
    assert_eq!(block.key.as_ref().map(|k| k.code.as_str()), Some("item.id"));
}

#[test]
fn parse_for_without_key() {
    // When
    let doc = parse_ok("{for i in items}<span>{i}</span>{/for}");

    // Then
    assert!(for_block(&doc.children[0]).key.is_none());
}

#[test]
fn parse_for_key_with_complex_expr() {
    // When
    let doc = parse_ok("{for (i, item) in items key=format!(\"{i}\")}<span>{item}</span>{/for}");

    // Then
    let block = for_block(&doc.children[0]);
    assert_eq!(block.iterable.code, "items");
    assert_eq!(
        block.key.as_ref().map(|k| k.code.as_str()),
        Some("format!(\"{i}\")")
    );
}

#[test]
fn parse_for_missing_close_is_refused() {
    // When
    let err = parse_err("{for i in items}<span>{i}</span>");

    // Then
    assert!(err.message.contains("{/for}"), "{err}");
}
