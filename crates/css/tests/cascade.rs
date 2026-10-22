//! The cascade: specificity, source order, state flags. Translated from
//! the cascade halves of sumi's `runtime/css/resolve_test.go`,
//! `resolve_focus_test.go`, `logical_test.go` and the matching tests of
//! `runtime/layout/resolve_styles_test.go`. sumi's `Resolve` returned
//! only declared properties; here an unmatched property is its initial
//! value, so "empty" becomes the initial value.

mod support;

use support::{add, computed, css, css_at, siblings};
use vapors_css::tree::{States, Tree};

const RED: &str = "rgb(255 0 0)";
const BLUE: &str = "rgb(0 0 255)";
const GREEN: &str = "rgb(0 128 0)";

#[test]
fn class_match_sets_the_property() {
    assert_eq!(
        css_at(".title { color: red; }", &["span.title"], "color"),
        RED
    );
}

#[test]
fn no_match_leaves_the_initial_value() {
    assert_eq!(
        css_at(".title { color: red; }", &["span.subtitle"], "color"),
        "rgb(0 0 0)"
    );
}

#[test]
fn no_rules_gives_initial_values_for_everything() {
    let (tree, leaf) = support::chain(&["span.anything"]);
    let style = computed("", &tree, leaf);
    assert_eq!(css(&style, "display"), "inline");
    assert_eq!(css(&style, "width"), "auto");
    assert_eq!(css(&style, "opacity"), "1");
}

#[test]
fn multiple_classes_match_multiple_rules() {
    let (tree, leaf) = support::chain(&["span.primary.strong"]);
    let style = computed(
        ".primary { color: blue; } .strong { font-weight: bold; }",
        &tree,
        leaf,
    );
    assert_eq!(css(&style, "color"), BLUE);
    assert_eq!(css(&style, "font-weight"), "bold");
}

#[test]
fn specificity_class_beats_type() {
    // Given: the type rule appears later but the class is more specific.
    let src = ".title { color: red; } span { color: blue; }";
    assert_eq!(css_at(src, &["span.title"], "color"), RED);
}

#[test]
fn specificity_id_beats_class() {
    let src = "#main { color: green; } .title { color: red; }";
    assert_eq!(css_at(src, &["span#main.title"], "color"), GREEN);
}

#[test]
fn equal_specificity_source_order_wins() {
    assert_eq!(
        css_at(
            ".a { color: red; } .b { color: blue; }",
            &["span.a.b"],
            "color"
        ),
        BLUE
    );
}

#[test]
fn lower_specificity_still_contributes_other_properties() {
    let (tree, leaf) = support::chain(&["span.title"]);
    let style = computed("span { opacity: 0.5; } .title { color: red; }", &tree, leaf);
    assert_eq!(css(&style, "color"), RED);
    assert_eq!(css(&style, "opacity"), "0.5");
}

#[test]
fn descendant_and_child_rules_resolve_through_the_tree() {
    assert_eq!(
        css_at(
            ".panel span { color: red; }",
            &["div.panel", "div", "span"],
            "color"
        ),
        RED
    );
    assert_eq!(
        css_at(
            ".panel > span { color: red; }",
            &["div.panel", "div", "span"],
            "color"
        ),
        "rgb(0 0 0)"
    );
}

#[test]
fn hover_rules_apply_with_the_flag() {
    // Given
    let src = ".btn:hover { background-color: cyan; } .btn { background-color: blue; }";
    let (mut tree, leaf) = support::chain(&["div.btn"]);
    assert_eq!(css(&computed(src, &tree, leaf), "background-color"), BLUE);

    // When
    tree.set_states(leaf, States::HOVER);

    // Then
    assert_eq!(
        css(&computed(src, &tree, leaf), "background-color"),
        "rgb(0 255 255)"
    );
}

#[test]
fn focus_rules_apply_with_the_flag() {
    let src = ".field:focus { border-top-color: cyan; } .field { border-top-color: white; }";
    let (mut tree, leaf) = support::chain(&["div.field"]);
    assert_eq!(
        css(&computed(src, &tree, leaf), "border-top-color"),
        "rgb(255 255 255)"
    );
    tree.set_states(leaf, States::FOCUS);
    assert_eq!(
        css(&computed(src, &tree, leaf), "border-top-color"),
        "rgb(0 255 255)"
    );
}

#[test]
fn where_has_zero_specificity_in_the_cascade() {
    // Given: the type rule (0,0,1) beats :where(.a) (0,0,0) despite order.
    let src = "span { color: blue; } :where(.a) { color: red; }";
    assert_eq!(css_at(src, &["span.a"], "color"), BLUE);
}

#[test]
fn is_keeps_argument_specificity_in_the_cascade() {
    let src = ":is(.a) { color: red; } span { color: blue; }";
    assert_eq!(css_at(src, &["span.a"], "color"), RED);
}

#[test]
fn runtime_siblings_stripe_with_nth_child() {
    // Given: three children as a {for} loop would build them.
    let (tree, kids) = siblings(&["span", "span", "span"]);
    let src = "root span:nth-child(odd) { color: red; }";

    // When
    let colors: Vec<String> = kids
        .iter()
        .map(|k| css(&computed(src, &tree, *k), "color"))
        .collect();

    // Then
    assert_eq!(colors, [RED, "rgb(0 0 0)", RED]);
}

#[test]
fn checked_and_disabled_pseudos_follow_the_flags() {
    // Given
    let mut tree = Tree::new("root");
    let root = tree.root();
    let checked = add(&mut tree, root, "input");
    let unchecked = add(&mut tree, root, "input");
    let disabled = add(&mut tree, root, "input");
    tree.set_states(checked, States::CHECKED.union(States::ENABLED));
    tree.set_states(unchecked, States::ENABLED);
    tree.set_states(disabled, States::DISABLED);
    let src = "input:enabled { color: white; } input:checked { color: green; } input:disabled { opacity: 0.5; }";

    // Then
    assert_eq!(css(&computed(src, &tree, checked), "color"), GREEN);
    assert_eq!(
        css(&computed(src, &tree, unchecked), "color"),
        "rgb(255 255 255)"
    );
    let d = computed(src, &tree, disabled);
    assert_eq!(css(&d, "opacity"), "0.5");
    assert_eq!(
        css(&d, "color"),
        "rgb(0 0 0)",
        "disabled must not match :enabled"
    );
}

#[test]
fn text_align_inherits_and_a_child_declaration_overrides() {
    // Given
    let mut tree = Tree::new("root");
    let root = tree.root();
    let left = add(&mut tree, root, "div.left");
    let span = add(&mut tree, left, "span");
    let plain = add(&mut tree, left, "p");
    let src = ".left { text-align: center; } .left span { text-align: right; }";

    // Then
    assert_eq!(css(&computed(src, &tree, plain), "text-align"), "center");
    assert_eq!(css(&computed(src, &tree, span), "text-align"), "right");
}
