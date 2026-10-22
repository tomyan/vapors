//! `@container` evaluation (`src/cascade/conditions.rs`). Translated from sumi `resolve_styles_test.go`.

mod support;

use support::{add, computed, css};
use vapors_css::tree::Tree;

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
