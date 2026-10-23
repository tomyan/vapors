//! `:global(...)` in a component's style: the compound holding it is
//! left unscoped, so a component can style `body` or an ancestor it did
//! not create. In a sheet that is not a component's it is refused.

mod support;

use support::{document, element, viewport};
use vapors_dom::{Dom, SCOPE_ATTRIBUTE};
use vapors_elements::{Document, ElementError};
use vapors_scene::Color;

fn painted_canvas(doc: &mut Document) -> Color {
    doc.update(viewport(100.0, 100.0))
        .expect("updates")
        .canvas()
}

#[test]
fn a_global_compound_in_a_scoped_style_matches_outside_the_scope() {
    // Given: body belongs to no component
    let mut doc = document();
    doc.add_style("a", ":global(body) { background-color: #0000ff; }");

    // When
    let canvas = painted_canvas(&mut doc);

    // Then
    assert_eq!(canvas, Color::from_rgba8(0, 0, 255, 255));
}

#[test]
fn the_other_compounds_of_a_selector_with_a_global_one_stay_scoped() {
    // Given: `:global(.x) .t` in scope a, .x in no scope; one .t in
    // scope a and one in scope b
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "x");
    let own = element(&mut doc, outer, "div", "t");
    doc.set_attribute(own, SCOPE_ATTRIBUTE, "a");
    let foreign = element(&mut doc, outer, "div", "t");
    doc.set_attribute(foreign, SCOPE_ATTRIBUTE, "b");
    doc.add_style(
        "a",
        ":global(.x) .t { height: 2px; background-color: #ff0000; }",
    );

    // When
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then: only scope a's .t is painted
    assert_eq!(
        painted.scene().items().len(),
        1,
        "{:?}",
        painted.scene().items()
    );
}

#[test]
fn global_in_a_sheet_that_is_not_a_components_is_refused_naming_its_line() {
    // Given
    let mut doc = document();

    // When
    let error = doc
        .add_stylesheet("app.css", "p { color: red; }\n:global(body) { margin: 0; }")
        .expect_err("refused");

    // Then
    let ElementError::Css { sheet, error } = error else {
        panic!("a CSS refusal, got {error:?}");
    };
    assert_eq!((sheet.as_str(), error.position.line), ("app.css", 2));
    assert!(
        error.message.contains("component's <style>"),
        "{}",
        error.message
    );
}

#[test]
fn a_global_subject_gains_its_one_attribute_on_the_last_scoped_compound() {
    // Given: scoped `.card :global(p)` (0,1,1), then a later global
    // `.card p` (0,1,1); scoped, it must count (0,2,1)
    let mut doc = document();
    let body = doc.body();
    let card = element(&mut doc, body, "div", "card");
    doc.set_attribute(card, SCOPE_ATTRIBUTE, "a");
    element(&mut doc, card, "p", "");
    doc.add_style(
        "a",
        ".card :global(p) { height: 2px; background-color: #ff0000; }",
    );
    doc.add_stylesheet("app.css", ".card p { background-color: #0000ff; }")
        .expect("parses");

    // When
    let painted = doc.update(viewport(100.0, 100.0)).expect("updates");

    // Then
    let colours: Vec<Color> = painted
        .scene()
        .items()
        .iter()
        .filter_map(|item| match item {
            vapors_scene::Item::Quad(quad) => Some(quad.background),
            _ => None,
        })
        .collect();
    assert_eq!(colours, [Color::from_rgba8(255, 0, 0, 255)]);
}

#[test]
fn global_below_the_top_level_is_refused_in_a_scoped_style() {
    // Given
    let mut doc = document();
    doc.add_style("a", "p {}\n.x:is(:global(.y)) { color: red; }");

    // When
    let errors = support::refusals(&mut doc);

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.starts_with("a:2:") && e.contains(":global()")),
        "{errors:?}"
    );
}

#[test]
fn global_below_the_top_level_is_refused_in_a_sheet() {
    // Given
    let mut doc = document();

    // When
    let error = doc
        .add_stylesheet("app.css", ".x:not(:global(.y)) { color: red; }")
        .expect_err("refused");

    // Then
    assert!(error.to_string().starts_with("app.css:1:"), "{error}");
}
