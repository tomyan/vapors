//! The tooltip a node shows: the nearest title from the node up, as
//! Chromium finds it, with whitespace as its tooltip shows it.

mod support;

use support::{document, element, text};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;

/// A `button` in a `div` titled `outer`, the button titled `inner`
/// when given, and a text node in the button.
fn nested(inner: Option<&str>) -> (Document, NodeId, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    doc.set_attribute(div, "title", "outer");
    let button = element(&mut doc, div, "button", "");
    if let Some(inner) = inner {
        doc.set_attribute(button, "title", inner);
    }
    let label = text(&mut doc, button, "Go");
    (doc, div, button, label)
}

#[test]
fn a_node_shows_its_nearest_titled_ancestors_title() {
    // Given
    let (doc, div, _, label) = nested(None);

    // When
    let shown = doc.tooltip_text(label);

    // Then
    assert_eq!(shown, Some((div, "outer".to_owned())));
}

#[test]
fn the_nearest_title_wins_over_an_outer_one() {
    // Given
    let (doc, _, button, label) = nested(Some("Open (o)"));

    // When
    let shown = doc.tooltip_text(label);

    // Then
    assert_eq!(shown, Some((button, "Open (o)".to_owned())));
}

#[test]
fn an_empty_or_blank_title_stops_the_walk_and_shows_nothing() {
    for title in ["", " \n\t ", "\u{a0}"] {
        // Given
        let (doc, _, _, label) = nested(Some(title));

        // When
        let shown = doc.tooltip_text(label);

        // Then
        assert_eq!(shown, None, "title {title:?}");
    }
}

#[test]
fn the_text_is_trimmed_keeps_its_line_breaks_and_widens_tabs() {
    // Given
    let (doc, _, button, _) = nested(Some("\u{a0} Undo\tCtrl+Z\nRedo  x \n"));

    // When
    let shown = doc.tooltip_text(button);

    // Then
    let expected = "Undo        Ctrl+Z\nRedo  x".to_owned();
    assert_eq!(shown, Some((button, expected)));
}

#[test]
fn no_title_anywhere_or_a_removed_node_shows_nothing() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let span = element(&mut doc, body, "span", "");
    let gone = element(&mut doc, body, "span", "");
    doc.set_attribute(gone, "title", "gone");
    doc.remove(gone);

    // Then
    assert_eq!(doc.tooltip_text(span), None);
    assert_eq!(doc.tooltip_text(gone), None);
}
