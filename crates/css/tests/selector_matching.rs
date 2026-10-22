//! Matching (`src/selector/matching.rs`). Translated from the matching halves of sumi `resolve_test.go` and `logical_test.go`.

mod support;

use support::{matches_chain, selector, siblings};
use vapors_css::selector::matches;
use vapors_css::tree::{States, Tree};

#[test]
fn class_match() {
    assert!(matches_chain(".title", &["span.title"]));
}

#[test]
fn element_match() {
    assert!(matches_chain("span", &["span"]));
}

#[test]
fn no_match_for_other_class() {
    assert!(!matches_chain(".title", &["span.subtitle"]));
}

#[test]
fn element_selector_does_not_match_wrong_tag() {
    assert!(!matches_chain("div", &["span"]));
}

#[test]
fn descendant_match_through_nesting() {
    assert!(matches_chain(
        ".panel span",
        &["root", "div.panel", "div", "span"]
    ));
}

#[test]
fn descendant_no_match_outside() {
    assert!(!matches_chain(".panel span", &["root", "div", "span"]));
}

#[test]
fn child_match() {
    assert!(matches_chain(
        ".panel > span",
        &["root", "div.panel", "span"]
    ));
}

#[test]
fn child_no_match_when_nested() {
    assert!(!matches_chain(
        ".panel > span",
        &["root", "div.panel", "div", "span"]
    ));
}

#[test]
fn universal_matches_everything() {
    assert!(matches_chain("*", &["span"]));
}

#[test]
fn id_selector() {
    assert!(matches_chain("#sidebar", &["div#sidebar"]));
    assert!(!matches_chain("#sidebar", &["div#main"]));
}

#[test]
fn tag_names_match_case_insensitively() {
    assert!(matches_chain("SPAN", &["span"]));
}

#[test]
fn adjacent_sibling_combinator() {
    let (tree, kids) = siblings(&["span.label", "span", "span"]);
    let sel = selector(".label + span");
    assert!(matches(&sel, &tree.node(kids[1])));
    assert!(!matches(&sel, &tree.node(kids[2])));
}

#[test]
fn general_sibling_combinator() {
    let (tree, kids) = siblings(&["span.label", "span", "span"]);
    let sel = selector(".label ~ span");
    assert!(matches(&sel, &tree.node(kids[1])));
    assert!(matches(&sel, &tree.node(kids[2])));
    assert!(!matches(&sel, &tree.node(kids[0])));
}

#[test]
fn sibling_then_descendant() {
    // Given: .a + .b span — span inside the div right after .a
    let (mut tree, boxes) = siblings(&["div.a", "div.b"]);
    let span = tree.append(boxes[1], "span");

    // When / Then
    assert!(matches(&selector(".a + .b span"), &tree.node(span)));
}

#[test]
fn not_excludes_matching() {
    assert!(matches_chain("span:not(.hint)", &["span"]));
    assert!(!matches_chain("span:not(.hint)", &["span.hint"]));
}

#[test]
fn is_matches_any_argument() {
    assert!(matches_chain(":is(.a, .b)", &["span.a"]));
    assert!(matches_chain(":is(.a, .b)", &["span.b"]));
    assert!(!matches_chain(":is(.a, .b)", &["span.c"]));
}

#[test]
fn where_matches_like_is() {
    assert!(matches_chain(":where(.a)", &["span.a"]));
}

#[test]
fn not_with_compound_argument() {
    assert!(!matches_chain(
        "div:not(.panel.active)",
        &["div.panel.active"]
    ));
    assert!(matches_chain("div:not(.panel.active)", &["div.panel"]));
}

#[test]
fn state_pseudo_classes_follow_the_element_flags() {
    for (sel, flag) in [
        (":hover", States::HOVER),
        (":focus", States::FOCUS),
        (":focus-visible", States::FOCUS_VISIBLE),
        (":focus-within", States::FOCUS_WITHIN),
        (":active", States::ACTIVE),
        (":checked", States::CHECKED),
        (":disabled", States::DISABLED),
        (":enabled", States::ENABLED),
    ] {
        // Given
        let mut tree = Tree::new("root");
        let root = tree.root();
        let el = tree.append(root, "button");
        let sel = selector(sel);
        assert!(!matches(&sel, &tree.node(el)), "{sel:?} without the flag");

        // When
        tree.set_states(el, flag);

        // Then
        assert!(matches(&sel, &tree.node(el)), "{sel:?} with the flag");
    }
}

#[test]
fn state_pseudo_on_an_ancestor_compound_matches_too() {
    // Given: sumi made such a rule inert; CSS matches it.
    let (mut tree, boxes) = siblings(&["div.btn"]);
    let span = tree.append(boxes[0], "span");
    tree.set_states(boxes[0], States::HOVER);

    // When / Then
    assert!(matches(&selector(".btn:hover span"), &tree.node(span)));
}

#[test]
fn is_with_complex_argument_matches() {
    // Given: sumi made combinator arguments never match.
    assert!(matches_chain(":is(.a > .b)", &["root", "div.a", "span.b"]));
    assert!(!matches_chain(
        ":is(.a > .b)",
        &["root", "div.a", "div", "span.b"]
    ));
}
