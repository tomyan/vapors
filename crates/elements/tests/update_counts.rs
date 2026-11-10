//! `Document::update_counts` (IU1 PR 2): each structural count is the
//! work its phase did in the last update, made again on an update with
//! nothing changed, and 0 when an earlier refusal stops the update
//! before its phase runs.

mod support;

use support::{document, element, refusals, text, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, UpdateCounts};

const CSS: &str =
    "body { margin: 0; } .swatch { width: 10px; height: 10px; background-color: red; }";

/// `html` > `body` > a red swatch and a div with text: 4 elements,
/// 5 boxes (the text run is one), 1 scene item per swatch and text run.
fn page() -> Document {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "swatch");
    let label = element(&mut doc, body, "div", "");
    text(&mut doc, label, "label");
    doc.add_stylesheet("app.css", CSS).expect("parses");
    doc
}

/// The counts of an update with nothing changed since the last.
fn idle_counts(doc: &mut Document) -> UpdateCounts {
    doc.update(viewport(100.0, 100.0)).expect("updates");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    assert_eq!(
        doc.elements_restyled(),
        0,
        "the second update restyles nothing"
    );
    doc.update_counts()
}

#[test]
fn a_document_never_updated_counts_nothing() {
    assert_eq!(page().update_counts(), UpdateCounts::default());
}

#[test]
fn the_style_phase_visits_every_element_on_an_update_with_nothing_changed() {
    assert_eq!(idle_counts(&mut page()).elements_visited, 4);
}

#[test]
fn a_style_attribute_that_does_not_parse_refuses_before_any_element_is_visited() {
    // Given: a successful update first, so a reset is what clears it
    let mut doc = page();
    doc.update(viewport(100.0, 100.0)).expect("updates");
    let body = doc.body();
    doc.set_attribute(body, "style", "color: ;;{");

    // When
    refusals(&mut doc);

    // Then
    assert_eq!(doc.update_counts(), UpdateCounts::default());
}

#[test]
fn every_box_is_built_on_an_update_with_nothing_changed() {
    assert_eq!(idle_counts(&mut page()).boxes_built, 5);
}

#[test]
fn a_tag_refused_by_the_style_phase_builds_no_box() {
    // Given
    let mut doc = page();
    let body = doc.body();
    element(&mut doc, body, "canvas", "");

    // When
    refusals(&mut doc);

    // Then
    let counts = doc.update_counts();
    assert_eq!((counts.elements_visited, counts.boxes_built), (5, 0));
}

#[test]
fn layout_asks_the_boxes_for_their_layout_on_an_update_with_nothing_changed() {
    // Given
    let mut doc = page();

    // When
    let counts = idle_counts(&mut doc);

    // Then: every answer a cache hit
    assert_eq!((doc.boxes_laid_out(), counts.layout_calls), (0, 1));
}

#[test]
fn a_calc_refused_by_layout_paints_nothing() {
    // Given: a division by zero only seen once % has a basis
    let mut doc = page();
    doc.add_stylesheet("calc.css", ".swatch { width: calc(50% / (1 - 1)); }")
        .expect("parses");

    // When
    refusals(&mut doc);

    // Then
    let counts = doc.update_counts();
    assert!(counts.layout_calls > 0, "{counts:?}");
    assert_eq!((counts.boxes_painted, counts.scene_items_built), (0, 0));
}

#[test]
fn paint_draws_every_box_and_rebuilds_every_item_on_an_update_with_nothing_changed() {
    let counts = idle_counts(&mut page());
    assert_eq!((counts.boxes_painted, counts.scene_items_built), (5, 2));
}

#[test]
fn a_scroll_thumb_is_not_a_painted_box() {
    // Given: html, body, a scrolling list and its 4 rows, thumb shown
    let mut list = support::scroll_list::list("overflow: auto;");

    // When
    list.doc
        .update(support::scroll_list::at(1.0))
        .expect("updates");

    // Then: 7 boxes; the thumb is a scene item, not a box
    assert_eq!(list.doc.update_counts().boxes_painted, 7);
}

#[test]
fn every_getter_describes_a_refused_update_not_the_update_before() {
    // Given: a successful update that restyled and laid out, then a calc
    // that layout refuses after restyling and laying out
    let mut doc = page();
    doc.update(viewport(100.0, 100.0)).expect("updates");
    assert!(doc.elements_restyled() > 0 && doc.boxes_laid_out() > 0);
    doc.add_stylesheet("calc.css", ".swatch { width: calc(50% / (1 - 1)); }")
        .expect("parses");

    // When
    refusals(&mut doc);

    // Then: the getters report the refused update's numbers
    let counts = doc.update_counts();
    assert_eq!(doc.elements_restyled(), counts.elements_restyled);
    assert_eq!(doc.boxes_laid_out(), counts.boxes_laid_out);
    assert!(counts.elements_restyled > 0, "{counts:?}");
    assert!(counts.boxes_laid_out > 0, "{counts:?}");
}

#[test]
fn a_style_attribute_refusal_resets_restyled_and_laid_out_to_zero() {
    // Given
    let mut doc = page();
    doc.update(viewport(100.0, 100.0)).expect("updates");
    let body = doc.body();
    doc.set_attribute(body, "style", "color: ;;{");

    // When
    refusals(&mut doc);

    // Then
    assert_eq!((doc.elements_restyled(), doc.boxes_laid_out()), (0, 0));
}
