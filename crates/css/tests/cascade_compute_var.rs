//! `var()` at compute time (`src/cascade/compute.rs`): custom properties, inheritance, shadowing, refusals and shorthands. Partly translated from sumi `resolve_styles_test.go`.

mod support;

use support::cascade_vars::CYAN;
use support::{add, cascade, chain, computed, css, env};
use vapors_css::tree::Tree;

#[test]
fn custom_properties_are_readable_on_the_computed_style() {
    let (tree, leaf) = chain(&["div.x"]);
    let style = computed("html { --accent: cyan; } .x { --gap: 4px; }", &tree, leaf);
    assert_eq!(
        style
            .custom("--accent")
            .map(vapors_css::serialize::component_values)
            .as_deref(),
        Some("cyan")
    );
    assert_eq!(
        style
            .custom("--gap")
            .map(vapors_css::serialize::component_values)
            .as_deref(),
        Some("4px")
    );
    assert_eq!(style.custom("--none"), None);
}

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
fn shorthands_with_var_substitute_then_expand() {
    // Given: review item 3; the var() path must expand shorthands too.
    let (tree, leaf) = support::chain(&["div.x"]);
    let src = "html { --x: 1px; --y: 2px; --c: red; --s: 2; }
        .x { padding: var(--x) var(--y); border: 1px solid var(--c); flex: var(--s); }";
    let style = computed(src, &tree, leaf);
    assert_eq!(css(&style, "padding-top"), "1px");
    assert_eq!(css(&style, "padding-right"), "2px");
    assert_eq!(css(&style, "border-top-width"), "1px");
    assert_eq!(css(&style, "border-left-style"), "solid");
    assert_eq!(css(&style, "border-bottom-color"), "rgb(255 0 0)");
    assert_eq!(css(&style, "flex-grow"), "2");
    assert_eq!(css(&style, "flex-shrink"), "1");
    assert_eq!(css(&style, "flex-basis"), "0px");
}
