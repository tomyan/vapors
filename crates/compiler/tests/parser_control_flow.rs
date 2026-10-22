//! Translated from sumi's `parser/template/parser_controlflow_test.go`.
//! sumi's loop clause is a Go range clause kept verbatim; here it is
//! `{for PATTERN in EXPR key=EXPR}`, so `Clause` becomes `pattern` and
//! `iterable`. Conditions and iterables are raw Rust (no auto-unwrap),
//! as in sumi.

mod common;

use common::{element, expr, if_block, literal, parse_err, parse_ok, text_parts};

#[test]
fn parse_if_basic() {
    // When
    let doc = parse_ok("{if x}<span>Yes</span>{/if}");

    // Then
    let block = if_block(&doc.children[0]);
    assert_eq!(block.condition.code, "x");
    assert_eq!(block.then.len(), 1);
    assert_eq!(text_parts(element(&block.then[0])), [literal("Yes")]);
    assert!(block.otherwise.is_none());
}

#[test]
fn parse_if_else() {
    // When
    let doc = parse_ok("{if count > 0}<span>Has items</span>{else}<span>Empty</span>{/if}");

    // Then
    let block = if_block(&doc.children[0]);
    assert_eq!(block.condition.code, "count > 0");
    assert_eq!(text_parts(element(&block.then[0])), [literal("Has items")]);
    let otherwise = block.otherwise.as_ref().expect("else branch");
    assert_eq!(text_parts(element(&otherwise[0])), [literal("Empty")]);
}

#[test]
fn parse_if_at_root() {
    // When
    let doc = parse_ok("<span>Before</span>{if visible}<span>Shown</span>{/if}<span>After</span>");

    // Then
    assert_eq!(doc.children.len(), 3);
    element(&doc.children[0]);
    if_block(&doc.children[1]);
    element(&doc.children[2]);
}

#[test]
fn parse_if_in_box() {
    // When
    let doc = parse_ok("<div>{if x}<span>Inside</span>{/if}</div>");

    // Then
    let div = element(&doc.children[0]);
    assert_eq!(if_block(&div.children[0]).condition.code, "x");
}

#[test]
fn parse_if_multiple_children() {
    // When
    let doc = parse_ok("{if x}<span>A</span><span>B</span><div><span>C</span></div>{/if}");

    // Then
    assert_eq!(if_block(&doc.children[0]).then.len(), 3);
}

#[test]
fn parse_if_nested_in_if() {
    // When
    let doc = parse_ok("{if x}{if y}<span>Both</span>{/if}{/if}");

    // Then
    let outer = if_block(&doc.children[0]);
    assert_eq!(outer.condition.code, "x");
    assert_eq!(if_block(&outer.then[0]).condition.code, "y");
}

#[test]
fn parse_if_missing_close_is_refused() {
    // When
    let err = parse_err("{if x}<span>Hello</span>");

    // Then
    assert!(err.message.contains("{/if}"), "{err}");
}

// vapors additions

#[test]
fn parse_for_without_in_is_refused() {
    // When
    let err = parse_err("{for items}<span>x</span>{/for}");

    // Then
    assert_eq!((err.line, err.col), (1, 1), "{err}");
    assert!(err.message.contains(" in "), "{err}");
}

#[test]
fn else_if_is_refused_with_the_nesting_pointer() {
    // When
    let err = parse_err("{if a}<span>a</span>{else if b}<span>b</span>{/if}");

    // Then
    assert_eq!((err.line, err.col), (1, 21), "{err}");
    assert!(err.message.contains("{else}"), "{err}");
}

#[test]
fn mismatched_block_terminator_is_refused() {
    // When
    let err = parse_err("{if a}<span>a</span>{/for}");

    // Then
    assert_eq!((err.line, err.col), (1, 21), "{err}");
    assert!(
        err.message.contains("{/if}") && err.message.contains("{/for}"),
        "{err}"
    );
}

#[test]
fn expression_in_text_is_distinct_from_a_block() {
    // When
    let doc = parse_ok("<p>{for_count} {iffy}</p>");

    // Then
    assert_eq!(
        text_parts(element(&doc.children[0])),
        [expr("for_count"), literal(" "), expr("iffy")]
    );
}
