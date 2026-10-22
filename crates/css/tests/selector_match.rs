//! Selector matching against a tree. Translated from the matching tests in
//! sumi's `runtime/css/resolve_test.go`, `resolve_attr_test.go`,
//! `structural_test.go` and `logical_test.go` (the cascade halves of those
//! files are translated in the cascade tests).

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

// --- attributes ---

#[test]
fn attribute_presence() {
    assert!(matches_chain("[focusable]", &["div[focusable=true]"]));
    assert!(!matches_chain("[focusable]", &["div"]));
}

#[test]
fn attribute_operators() {
    for (sel, value, want) in [
        ("[a=hello]", "hello", true),
        ("[a=hello]", "hell", false),
        ("[a^=he]", "hello", true),
        ("[a^=lo]", "hello", false),
        ("[a$=lo]", "hello", true),
        ("[a$=he]", "hello", false),
        ("[a*=ell]", "hello", true),
        ("[a*=xyz]", "hello", false),
        ("[a~=b]", "a b c", true),
        ("[a~=d]", "a b c", false),
        ("[a|=en]", "en-GB", true),
        ("[a|=en]", "en", true),
        ("[a|=en]", "enx", false),
        ("[a=HELLO i]", "hello", true),
        ("[a=HELLO]", "hello", false),
    ] {
        // Given
        let mut tree = Tree::new("root");
        let root = tree.root();
        let el = tree.append(root, "div");
        tree.set_attribute(el, "a", value);

        // When / Then
        assert_eq!(
            matches(&selector(sel), &tree.node(el)),
            want,
            "{sel} vs {value:?}"
        );
    }
}

// --- structural ---

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

// --- :not(), :is(), :where() ---

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

// --- state flags ---

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
