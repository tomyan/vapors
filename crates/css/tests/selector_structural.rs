//! Structural pseudo-classes and `:has()` (`src/selector/structural.rs`). Translated from sumi `structural_test.go`.

mod support;

use support::{selector, siblings};
use vapors_css::selector::matches;
use vapors_css::tree::Tree;

#[test]
fn first_and_last_child() {
    // Given
    let (tree, kids) = siblings(&["span", "span", "span"]);
    let first = selector("span:first-child");
    let last = selector("span:last-child");

    // When / Then
    assert!(matches(&first, &tree.node(kids[0])));
    assert!(!matches(&first, &tree.node(kids[1])) && !matches(&last, &tree.node(kids[1])));
    assert!(matches(&last, &tree.node(kids[2])));
}

#[test]
fn only_child() {
    let (single, one) = siblings(&["span"]);
    assert!(matches(&selector("span:only-child"), &single.node(one[0])));
    let (three, kids) = siblings(&["span", "span", "span"]);
    assert!(!matches(&selector("span:only-child"), &three.node(kids[0])));
}

#[test]
fn nth_child_odd() {
    // Given
    let (tree, kids) = siblings(&["span", "span", "span"]);
    let odd = selector("span:nth-child(odd)");

    // When / Then
    let got: Vec<bool> = kids.iter().map(|k| matches(&odd, &tree.node(*k))).collect();
    assert_eq!(got, [true, false, true]);
}

#[test]
fn nth_last_child() {
    let (tree, kids) = siblings(&["span", "span", "span"]);
    let sel = selector("span:nth-last-child(1)");
    assert!(matches(&sel, &tree.node(kids[2])));
    assert!(!matches(&sel, &tree.node(kids[0])));
}

#[test]
fn of_type_variants() {
    // Given: div, span, div; of-type indices count per tag.
    let (tree, kids) = siblings(&["div", "span", "div"]);

    // When / Then
    assert!(matches(&selector("div:first-of-type"), &tree.node(kids[0])));
    assert!(!matches(
        &selector("div:first-of-type"),
        &tree.node(kids[2])
    ));
    assert!(matches(&selector("div:last-of-type"), &tree.node(kids[2])));
    assert!(matches(&selector("span:only-of-type"), &tree.node(kids[1])));
    assert!(matches(
        &selector("div:nth-of-type(2)"),
        &tree.node(kids[2])
    ));
    assert!(matches(
        &selector("div:nth-last-of-type(2)"),
        &tree.node(kids[0])
    ));
}

#[test]
fn empty_pseudo() {
    // Given
    let mut tree = Tree::new("root");
    let root = tree.root();
    let empty = tree.append(root, "div");
    let with_text = tree.append(root, "div");
    tree.append_text(with_text, "hi");
    let with_child = tree.append(root, "div");
    tree.append(with_child, "span");

    // When / Then
    let sel = selector("div:empty");
    assert!(matches(&sel, &tree.node(empty)));
    assert!(!matches(&sel, &tree.node(with_text)));
    assert!(!matches(&sel, &tree.node(with_child)));
}

#[test]
fn root_pseudo_matches_the_tree_root_only() {
    let (tree, kids) = siblings(&["div"]);
    assert!(matches(&selector(":root"), &tree.node(tree.root())));
    assert!(!matches(&selector(":root"), &tree.node(kids[0])));
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
