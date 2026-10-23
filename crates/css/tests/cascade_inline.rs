//! The `style` attribute (Cascade Level 4 section 6.2): its declarations
//! are author declarations that beat any selector's, below an
//! `!important` sheet declaration unless they are important themselves.

mod support;

use support::cascade::{BLUE, RED};
use support::{chain, computed, css, env, sheet};
use vapors_css::cascade::{Cascade, INLINE_SHEET};
use vapors_css::property::PropertyId;
use vapors_css::tree::Tree;

/// The leaf of `div > p#a.b.c` with `style`.
fn styled(style: &str) -> (Tree, vapors_css::tree::NodeId) {
    let (mut tree, leaf) = chain(&["div", "p#a.b.c"]);
    tree.set_attribute(leaf, "style", style);
    (tree, leaf)
}

#[test]
fn an_inline_declaration_beats_any_selector() {
    // Given
    let (tree, leaf) = styled("color: blue");

    // When
    let style = computed("#a#a.b.c { color: red; }", &tree, leaf);

    // Then
    assert_eq!(css(&style, "color"), BLUE);
}

#[test]
fn an_important_sheet_declaration_beats_a_normal_inline_one() {
    // Given
    let (tree, leaf) = styled("color: blue");

    // When
    let style = computed("p { color: red !important; }", &tree, leaf);

    // Then
    assert_eq!(css(&style, "color"), RED);
}

#[test]
fn an_important_inline_declaration_beats_an_important_sheet_one() {
    // Given
    let (tree, leaf) = styled("color: blue !important");

    // When
    let style = computed("#a { color: red !important; }", &tree, leaf);

    // Then
    assert_eq!(css(&style, "color"), BLUE);
}

#[test]
fn an_inline_declaration_names_the_inline_sheet_and_its_column() {
    // Given
    let (tree, leaf) = styled("width: 4px; color: blue");

    // When
    let style = computed("p { color: red; }", &tree, leaf);

    // Then
    let color = PropertyId::from_name("color").expect("a longhand");
    assert_eq!(style.declared_in(color), Some(INLINE_SHEET));
    assert_eq!(
        style.declared_at(color).map(|p| (p.line, p.column)),
        Some((1, 13))
    );
}

#[test]
fn an_inline_style_that_does_not_parse_is_refused_naming_the_attribute() {
    // Given
    let (tree, _leaf) = styled("colour: blue");
    let cascade = Cascade::new().author(sheet("p { color: red; }"));

    // When
    let error = cascade
        .compute_tree(&tree.node(tree.root()), &env())
        .expect_err("refused");

    // Then
    assert!(
        error.message.contains("style attribute"),
        "{}",
        error.message
    );
    assert!(error.message.contains("colour"), "{}", error.message);
}
