//! Custom properties, `calc()`, `@container` and `@supports` through the
//! cascade. Translated from sumi's `runtime/layout/resolve_styles_test.go`
//! (the A11 to A13 groups).

mod support;

use support::{add, cascade, computed, css, env, refusal};
use vapors_css::tree::Tree;

const CYAN: &str = "rgb(0 255 255)";

#[test]
fn var_inherits_down_the_tree() {
    // Given: a theme variable on root, used two levels down.
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div");
    let text = add(&mut tree, div, "span.accented");
    let src = "root { --accent: cyan; } .accented { color: var(--accent); }";

    // Then
    assert_eq!(css(&computed(src, &tree, text), "color"), CYAN);
}

#[test]
fn var_shadowing() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let over = add(&mut tree, root, "div.override");
    let inner = add(&mut tree, over, "span.accented");
    let outer = add(&mut tree, root, "span.accented");
    let src = "root { --accent: cyan; } .override { --accent: magenta; } .accented { color: var(--accent); }";
    assert_eq!(css(&computed(src, &tree, inner), "color"), "rgb(255 0 255)");
    assert_eq!(css(&computed(src, &tree, outer), "color"), CYAN);
}

#[test]
fn var_in_layout_property() {
    let (tree, leaf) = support::chain(&["div.sized"]);
    let src = "html { --panel-width: 30px; } .sized { width: var(--panel-width); }";
    assert_eq!(css(&computed(src, &tree, leaf), "width"), "30px");
}

#[test]
fn unresolved_var_is_refused_naming_the_variable_and_declaration() {
    // Given
    let (tree, leaf) = support::chain(&["span.x"]);
    let src = ".x {\n  color: var(--nope);\n}";

    // When
    let err = cascade(src)
        .compute(&tree.node(leaf), None, &env())
        .unwrap_err();

    // Then
    assert_eq!((err.position.line, err.position.column), (2, 3));
    assert!(
        err.message.contains("--nope") && err.message.contains("color"),
        "{err}"
    );
}

#[test]
fn var_value_that_fails_the_grammar_is_refused() {
    let (tree, _leaf) = support::chain(&["span.x"]);
    let src = "html { --w: red; } .x { width: var(--w); }";
    let err = cascade(src)
        .compute_tree(&tree.node(tree.root()), &env())
        .unwrap_err();
    assert!(
        err.message.contains("width") && err.message.contains("red"),
        "{err}"
    );
}

#[test]
fn custom_property_can_use_another_custom_property() {
    let (tree, leaf) = support::chain(&["span.x"]);
    let src =
        "html { --base: 4px; --double: calc(var(--base) * 2); } .x { padding-top: var(--double); }";
    assert_eq!(css(&computed(src, &tree, leaf), "padding-top"), "8px");
}

#[test]
fn calc_without_percent_resolves_to_px() {
    let (tree, leaf) = support::chain(&["div.x"]);
    let style = computed(
        ".x { width: calc(10px + 5px); row-gap: min(3px, 2px); }",
        &tree,
        leaf,
    );
    assert_eq!(css(&style, "width"), "15px");
    assert_eq!(css(&style, "row-gap"), "2px");
}

#[test]
fn calc_with_percent_defers_to_layout() {
    // Given: calc against the containing block stays a calc.
    let (tree, leaf) = support::chain(&["div.x"]);
    let style = computed(".x { width: calc(100% - 10px); }", &tree, leaf);
    assert_eq!(css(&style, "width"), "calc(100% - 10px)");
}

#[test]
fn container_query_matches_the_laid_out_ancestor() {
    // Given: a child styled red only when its container is at least 40px wide.
    let mut tree = Tree::new("root");
    let root = tree.root();
    let panel = add(&mut tree, root, "div.panel");
    let label = add(&mut tree, panel, "span.label");
    let src = "@container (min-width: 40px) { .label { color: red; } }";

    // Before layout no size is known: the query does not match.
    assert_eq!(css(&computed(src, &tree, label), "color"), "rgb(0 0 0)");

    // When: layout stamps the size.
    tree.set_container_size(panel, 50.0, 10.0);

    // Then
    assert_eq!(css(&computed(src, &tree, label), "color"), "rgb(255 0 0)");
}

#[test]
fn container_query_below_threshold() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let panel = add(&mut tree, root, "div");
    let label = add(&mut tree, panel, "span.label");
    tree.set_container_size(panel, 20.0, 10.0);
    let src = "@container (min-width: 40px) { .label { color: red; } }";
    assert_eq!(css(&computed(src, &tree, label), "color"), "rgb(0 0 0)");
}

#[test]
fn supports_checks_property_and_value() {
    // Given: sumi checked the property name only; vapors parses the value.
    let (tree, leaf) = support::chain(&["span.x"]);
    let src = "@supports (display: grid) { .x { color: red; } }
        @supports (display: table-cell) { .x { color: blue; } }
        @supports (border-title: x) { .x { color: green; } }
        @supports (--custom: anything) { .x { opacity: 0.5; } }
        @supports not (display: grid) { .x { width: 1px; } }";
    let style = computed(src, &tree, leaf);
    assert_eq!(css(&style, "color"), "rgb(255 0 0)");
    assert_eq!(css(&style, "opacity"), "0.5");
    assert_eq!(css(&style, "width"), "auto");
}

#[test]
fn supports_with_and_or_and_malformed_refused() {
    let (tree, leaf) = support::chain(&["span.x"]);
    let src = "@supports (display: grid) and (color: red) { .x { color: red; } } @supports (display: grid) or (nope: 1) { .x { opacity: 0.5; } }";
    let style = computed(src, &tree, leaf);
    assert_eq!(css(&style, "color"), "rgb(255 0 0)");
    assert_eq!(css(&style, "opacity"), "0.5");
    assert!(
        refusal("@supports display: grid { }")
            .message
            .contains("@supports")
    );
}
