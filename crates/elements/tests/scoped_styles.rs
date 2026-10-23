//! A component's style, added with `Dom::add_style` under its scope,
//! styles only elements tagged with that scope (`data-wisp`), with
//! every compound of a selector in scope, and gains one attribute's
//! specificity (Svelte's rule).

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::{Dom, NodeId, SCOPE_ATTRIBUTE};
use vapors_elements::Document;
use vapors_scene::{Color, Item};

fn red() -> Color {
    Color::from_rgba8(255, 0, 0, 255)
}

fn blue() -> Color {
    Color::from_rgba8(0, 0, 255, 255)
}

/// An element appended to `parent`, with `class`, tagged with `scope`
/// when it is not empty, as a component's `mount` tags it.
fn scoped(doc: &mut Document, parent: NodeId, scope: &str, class: &str) -> NodeId {
    let node = element(doc, parent, "div", class);
    if !scope.is_empty() {
        doc.set_attribute(node, SCOPE_ATTRIBUTE, scope);
    }
    node
}

/// The background of every quad painted, in paint order.
fn fills(doc: &mut Document) -> Vec<Color> {
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");
    painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            Item::Quad(quad) => Some(quad.background),
            _ => None,
        })
        .collect()
}

#[test]
fn a_scoped_rule_styles_its_own_elements_and_no_others() {
    // Given: the same class in scope a, in scope b and in no scope
    let mut doc = document();
    let body = doc.body();
    scoped(&mut doc, body, "a", "t");
    scoped(&mut doc, body, "b", "t");
    scoped(&mut doc, body, "", "t");
    doc.add_style("a", ".t { height: 2px; background-color: #ff0000; }");

    // When
    let fills = fills(&mut doc);

    // Then: only scope a's element is painted
    assert_eq!(fills, [red()]);
}

#[test]
fn every_compound_of_a_scoped_selector_must_be_in_scope() {
    // Given: `.p .t` in scope a, where one .t's .p is in scope b
    let mut doc = document();
    let body = doc.body();
    let foreign = scoped(&mut doc, body, "b", "p");
    scoped(&mut doc, foreign, "a", "t");
    let own = scoped(&mut doc, body, "a", "p");
    scoped(&mut doc, own, "a", "t");
    doc.add_style("a", ".p .t { height: 2px; background-color: #ff0000; }");

    // When
    let fills = fills(&mut doc);

    // Then: only the .t under scope a's .p
    assert_eq!(fills, [red()]);
}

#[test]
fn a_scoped_rule_gains_one_attributes_specificity_over_the_same_global_rule() {
    // Given: scoped `.p .t` (0,3,0 once scoped), then a later global
    // `.p .t` (0,2,0)
    let mut doc = document();
    let body = doc.body();
    let parent = scoped(&mut doc, body, "a", "p");
    scoped(&mut doc, parent, "a", "t");
    doc.add_style("a", ".p .t { height: 2px; background-color: #ff0000; }");
    doc.add_stylesheet("app.css", ".p .t { background-color: #0000ff; }")
        .expect("parses");

    // When
    let fills = fills(&mut doc);

    // Then
    assert_eq!(fills, [red()]);
}

#[test]
fn a_scoped_rule_gains_only_one_attributes_specificity_however_many_compounds() {
    // Given: scoped `.p .t` (0,3,0, not 0,4,0), then a later global
    // `.t.x.y` (0,3,0)
    let mut doc = document();
    let body = doc.body();
    let parent = scoped(&mut doc, body, "a", "p");
    scoped(&mut doc, parent, "a", "t x y");
    doc.add_style("a", ".p .t { height: 2px; background-color: #ff0000; }");
    doc.add_stylesheet("app.css", ".t.x.y { background-color: #0000ff; }")
        .expect("parses");

    // When
    let fills = fills(&mut doc);

    // Then: equal specificity, so the later global rule wins
    assert_eq!(fills, [blue()]);
}

#[test]
fn a_scope_added_again_is_not_added_twice() {
    // Given: scope a, a later global rule over it, then scope a again,
    // as a second mount of the same component adds it
    let mut doc = document();
    let body = doc.body();
    scoped(&mut doc, body, "a", "t x");
    doc.add_style("a", ".t { height: 2px; background-color: #ff0000; }");
    doc.add_stylesheet("app.css", ".t.x { background-color: #0000ff; }")
        .expect("parses");
    doc.add_style("a", ".t { height: 2px; background-color: #ff0000; }");

    // When
    let fills = fills(&mut doc);

    // Then: the global rule still comes after scope a's
    assert_eq!(fills, [blue()]);
}

#[test]
fn a_scoped_style_that_does_not_parse_is_refused_by_the_update_naming_its_scope() {
    // Given
    let mut doc = document();
    doc.add_style("demos/panel.wisp", ".t { colour: red; }");

    // When
    let found = refusals(&mut doc);

    // Then
    assert!(
        found.iter().any(|e| e.contains("demos/panel.wisp")),
        "{found:?}"
    );
}
