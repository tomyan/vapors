//! SVG presentation attributes (`src/cascade/presentation.rs`): `fill`,
//! `stroke` and `stroke-width` on an `<svg>` or `<path>` are author
//! declarations below every author rule, as Chromium ranks them; on any
//! other element they are not styles; a value that is not one of the
//! property's is refused naming the attribute; and `fill` and `stroke`
//! keep `currentcolor` as their computed value, so a path inherits the
//! keyword, not its svg's colour.

mod support;

use support::{add, chain, computed, css, env, sheet};
use vapors_css::cascade::{Cascade, PRESENTATION_SHEET};
use vapors_css::property::PropertyId;
use vapors_css::tree::{NodeId, Tree};

/// `svg > path` with the path's presentation `attributes`: the tree,
/// the svg and the path.
fn svg_path(attributes: &[(&str, &str)]) -> (Tree, NodeId, NodeId) {
    let mut tree = Tree::new("html");
    let root = tree.root();
    let svg = add(&mut tree, root, "svg");
    let leaf = add(&mut tree, svg, "path");
    for (name, value) in attributes {
        tree.set_attribute(leaf, name, value);
    }
    (tree, svg, leaf)
}

fn path(attributes: &[(&str, &str)]) -> (Tree, NodeId) {
    let (tree, _, leaf) = svg_path(attributes);
    (tree, leaf)
}

fn refused(tree: &Tree) -> String {
    let error = Cascade::new()
        .author(sheet(""))
        .compute_tree(&tree.node(tree.root()), &env())
        .expect_err("refused");
    error.message
}

#[test]
fn an_attribute_with_no_rule_is_the_computed_value() {
    // Given
    let (tree, leaf) = path(&[("fill", "none"), ("stroke", "red"), ("stroke-width", "1.5")]);

    // When
    let style = computed("", &tree, leaf);

    // Then
    assert_eq!(css(&style, "fill"), "none");
    assert_eq!(css(&style, "stroke"), "rgb(255 0 0)");
    assert_eq!(css(&style, "stroke-width"), "1.5px");
    let stroke = PropertyId::from_name("stroke").expect("table");
    assert_eq!(style.declared_in(stroke), Some(PRESENTATION_SHEET));
}

#[test]
fn every_author_rule_beats_an_attribute_even_the_universal_one() {
    // Given
    let (tree, leaf) = path(&[("stroke-width", "1.5"), ("fill", "none")]);

    // When
    let style = computed("path { stroke-width: 2px } * { fill: blue }", &tree, leaf);

    // Then
    assert_eq!(css(&style, "stroke-width"), "2px");
    assert_eq!(css(&style, "fill"), "rgb(0 0 255)");
}

#[test]
fn the_style_attribute_beats_an_attribute() {
    // Given
    let (mut tree, leaf) = path(&[("stroke-width", "1.5")]);
    tree.set_attribute(leaf, "style", "stroke-width: 3");

    // When
    let style = computed("", &tree, leaf);

    // Then
    assert_eq!(css(&style, "stroke-width"), "3px");
}

#[test]
fn an_attribute_on_the_svg_is_inherited_by_its_path() {
    // Given
    let (mut tree, svg, leaf) = svg_path(&[]);
    tree.set_attribute(svg, "stroke-width", "2");

    // When
    let style = computed("", &tree, leaf);

    // Then
    assert_eq!(css(&style, "stroke-width"), "2px");
}

#[test]
fn the_attributes_on_an_element_that_is_not_svg_are_not_styles() {
    // Given: Chromium maps them on SVG elements only
    let (mut tree, leaf) = chain(&["div"]);
    tree.set_attribute(leaf, "fill", "red");

    // When
    let style = computed("", &tree, leaf);

    // Then: fill's initial value
    assert_eq!(css(&style, "fill"), "rgb(0 0 0)");
}

#[test]
fn an_attribute_that_is_not_one_value_of_its_property_is_refused_naming_it() {
    for (name, value, says) in [
        ("stroke-width", "thick", ""),
        ("stroke-width", "-1", ""),
        (
            "stroke-width",
            "50%",
            "a percentage stroke-width is outside vapors' subset",
        ),
        ("fill", "bluish", ""),
        (
            "fill",
            "red !important",
            "an attribute holds one fill value",
        ),
        (
            "fill",
            "none; color: red",
            "an attribute holds one fill value",
        ),
    ] {
        // Given
        let (tree, _) = path(&[(name, value)]);

        // When
        let message = refused(&tree);

        // Then
        let start = format!("the {name} attribute of <path> is {value:?}: {says}");
        assert!(message.starts_with(&start), "{name}={value:?}: {message}");
    }
}

#[test]
fn a_var_in_an_attribute_is_substituted_as_chromium_substitutes_it() {
    // Given: Chromium 155 paints fill="var(--c)" in --c
    let (tree, leaf) = path(&[("fill", "var(--c)")]);

    // When
    let style = computed("svg { --c: blue }", &tree, leaf);

    // Then
    assert_eq!(css(&style, "fill"), "rgb(0 0 255)");
}

#[test]
fn a_path_inherits_currentcolor_and_keeps_the_keyword() {
    // Given: the svg strokes in currentColor and is red; the path is blue
    let (mut tree, svg, leaf) = svg_path(&[]);
    tree.set_attribute(svg, "stroke", "currentColor");

    // When
    let style = computed("svg { color: red } path { color: blue }", &tree, leaf);

    // Then: the keyword, which the path resolves with its own blue
    assert_eq!(css(&style, "stroke"), "currentcolor");
}
