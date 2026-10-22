//! Selector behaviour sumi lacks: `:has()`, complex arguments in `:is()`
//! and `:not()`, their specificity, and the refusals. vapors-specific.

mod support;

use support::{matches_chain, refusal, selector, siblings};
use vapors_css::selector::{Specificity, matches, parse_selector_list};

#[test]
fn is_with_complex_argument_matches() {
    // Given: sumi made combinator arguments never match.
    assert!(matches_chain(":is(.a > .b)", &["root", "div.a", "span.b"]));
    assert!(!matches_chain(
        ":is(.a > .b)",
        &["root", "div.a", "div", "span.b"]
    ));
}

#[test]
fn has_descendant_and_child() {
    // Given
    let (mut tree, boxes) = siblings(&["div.a", "div.b"]);
    let inner = tree.append(boxes[0], "div");
    support::add(&mut tree, inner, "span.deep");
    tree.append(boxes[1], "span");

    // When / Then
    assert!(matches(&selector("div:has(.deep)"), &tree.node(boxes[0])));
    assert!(!matches(
        &selector("div:has(> .deep)"),
        &tree.node(boxes[0])
    ));
    assert!(matches(&selector("div:has(> span)"), &tree.node(boxes[1])));
    assert!(!matches(
        &selector("div:has(span.deep)"),
        &tree.node(boxes[1])
    ));
}

#[test]
fn has_sibling_forms() {
    // Given
    let (tree, kids) = siblings(&["span.a", "span.b", "span.c"]);

    // When / Then
    assert!(matches(&selector(".a:has(+ .b)"), &tree.node(kids[0])));
    assert!(!matches(&selector(".a:has(+ .c)"), &tree.node(kids[0])));
    assert!(matches(&selector(".a:has(~ .c)"), &tree.node(kids[0])));
    assert!(!matches(&selector(".c:has(~ .a)"), &tree.node(kids[2])));
}

#[test]
fn where_has_zero_specificity_and_is_keeps_its_argument() {
    assert_eq!(
        selector(":where(.a #b)").specificity(),
        Specificity::default()
    );
    assert_eq!(
        selector(":is(.a, #b)").specificity(),
        Specificity {
            ids: 1,
            classes: 0,
            types: 0
        }
    );
    assert_eq!(
        selector("span:not(.a.b, span)").specificity(),
        Specificity {
            ids: 0,
            classes: 2,
            types: 1
        }
    );
    assert_eq!(
        selector("div:has(> .x)").specificity(),
        Specificity {
            ids: 0,
            classes: 1,
            types: 1
        }
    );
}

#[test]
fn unknown_pseudo_class_is_refused_naming_it() {
    // Given: sumi let `:future-thing` parse and never match.
    let err = refusal("span:future-thing { color: red; }");

    // Then
    assert_eq!(err.position.column, 5);
    assert!(err.message.contains("future-thing"), "{err}");
}

#[test]
fn pseudo_elements_are_refused_as_unsupported() {
    for text in [".item::before", "a::after", "li:before"] {
        let err = refusal(&format!("{text} {{ color: cyan; }}"));
        assert!(err.message.contains("pseudo-element"), "{text}: {err}");
    }
}

#[test]
fn malformed_selectors_are_refused_with_a_position() {
    for (text, column) in [
        (".", 1),
        ("div >", 5),
        ("> div", 1),
        ("[a", 1),
        ("[=v]", 1),
        ("div:nth-child(2m)", 4),
        ("div:not()", 4),
        ("a,,b", 3),
        (":has(:has(a))", 6),
    ] {
        let err = parse_selector_list(text).expect_err(text);
        assert_eq!(err.position.column, column, "{text}: {err}");
    }
}

#[test]
fn empty_selector_in_list_is_refused() {
    let err = refusal("a, { color: red; }");
    assert!(err.message.contains("empty selector"), "{err}");
}

#[test]
fn has_nested_at_any_depth_is_refused() {
    // Given: review item 14.
    for text in [
        ":has(:is(:has(a)))",
        ":has(.a :not(:has(b)))",
        ":has(:is(.x, :where(:has(c))))",
    ] {
        let err = parse_selector_list(text).expect_err(text);
        assert!(
            err.message.contains(":has() inside :has()"),
            "{text}: {err}"
        );
    }
}
