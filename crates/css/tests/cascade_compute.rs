//! Computed values (`src/cascade/compute.rs`): inheritance, units, colours, `var()`. Partly translated from sumi `resolve_styles_test.go`.

mod support;

use support::cascade::RED;
use support::{add, chain, computed, css, env, sheet};
use vapors_css::cascade::Cascade;
use vapors_css::tree::Tree;
use vapors_css::value::ColorScheme;

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

#[test]
fn inherited_properties_inherit_and_others_take_initial() {
    // Given
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span");
    let src = ".p { color: red; opacity: 0.5; font-size: 20px; }";

    // Then
    let style = computed(src, &tree, span);
    assert_eq!(css(&style, "color"), RED);
    assert_eq!(css(&style, "font-size"), "20px");
    assert_eq!(css(&style, "opacity"), "1");
}

#[test]
fn css_wide_keywords() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span.c");
    let src = ".p { color: red; opacity: 0.5; } .c { color: initial; opacity: inherit; width: unset; font-size: unset; }";
    let style = computed(src, &tree, span);
    assert_eq!(css(&style, "color"), "rgb(0 0 0)");
    assert_eq!(css(&style, "opacity"), "0.5");
    assert_eq!(css(&style, "width"), "auto");
    assert_eq!(css(&style, "font-size"), "16px");
}

#[test]
fn relative_units_resolve_against_font_sizes_and_viewport() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span.c");
    let src = "root { font-size: 10px; } .p { font-size: 2em; } .c { font-size: 150%; width: 2em; height: 1rem; margin-top: 10vw; margin-left: 10vh; padding-top: 2ch; }";
    let style = computed(src, &tree, span);
    assert_eq!(
        css(&style, "font-size"),
        "30px",
        "150% of the parent's 20px"
    );
    assert_eq!(
        css(&style, "width"),
        "60px",
        "em of the element's own font size"
    );
    assert_eq!(css(&style, "height"), "10px", "rem is the root's font size");
    assert_eq!(css(&style, "margin-top"), "80px");
    assert_eq!(css(&style, "margin-left"), "60px");
    assert_eq!(
        css(&style, "padding-top"),
        "30px",
        "ch is zero_advance_em (0.5) of 30px"
    );
    assert_eq!(style.font_size(), 30.0);
}

#[test]
fn percentages_other_than_font_size_stay_percentages() {
    let (tree, leaf) = chain(&["div.x"]);
    let style = computed(
        ".x { width: 50%; line-height: 150%; font-size: 20px; }",
        &tree,
        leaf,
    );
    assert_eq!(css(&style, "width"), "50%");
    assert_eq!(
        css(&style, "line-height"),
        "30px",
        "line-height % resolves against own font size"
    );
}

#[test]
fn current_color_and_light_dark_resolve_at_compute() {
    let mut tree = Tree::new("root");
    let root = tree.root();
    let div = add(&mut tree, root, "div.p");
    let span = add(&mut tree, div, "span.c");
    let src = ".p { color: red; } .c { border-top-color: currentColor; background-color: light-dark(white, black); color: currentColor; }";
    let cascade = Cascade::new().author(sheet(src));
    let mut e = env();
    e.color_scheme = ColorScheme::Dark;
    let styles = cascade
        .compute_tree(&tree.node(root), &e)
        .expect("computes");
    let (_, style) = styles.iter().find(|(n, _)| n.id() == span).expect("span");
    assert_eq!(css(style, "border-top-color"), RED);
    assert_eq!(css(style, "background-color"), "rgb(0 0 0)");
    assert_eq!(
        css(style, "color"),
        RED,
        "color: currentColor is the parent's colour"
    );
    assert_eq!(style.color().to_rgba8(), [255, 0, 0, 255]);
}
