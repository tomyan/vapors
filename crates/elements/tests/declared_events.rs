//! `declare_events` checks every element again only when it declares an
//! event not declared before: every component mount calls it, and a
//! repeat changes no element's refusal.

mod support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// `html` > `body` > a div, with `optionchoose` declared, updated.
fn declared_page() -> Document {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "");
    doc.declare_events(&["optionchoose"]);
    doc.update(viewport(100.0, 100.0)).expect("updates");
    doc
}

fn visited_after_update(doc: &mut Document) -> usize {
    doc.update(viewport(100.0, 100.0)).expect("updates");
    doc.update_counts().elements_visited
}

#[test]
fn declaring_a_new_event_checks_every_element_again() {
    // Given
    let mut doc = declared_page();

    // When
    doc.declare_events(&["optionchoose", "pick"]);

    // Then: html, body and the div
    assert_eq!(visited_after_update(&mut doc), 3);
}

#[test]
fn declaring_an_event_already_declared_checks_no_element() {
    // Given
    let mut doc = declared_page();

    // When
    doc.declare_events(&["optionchoose"]);

    // Then
    assert_eq!(visited_after_update(&mut doc), 0);
}

#[test]
fn declaring_no_event_checks_no_element() {
    // Given
    let mut doc = declared_page();

    // When
    doc.declare_events(&[]);

    // Then
    assert_eq!(visited_after_update(&mut doc), 0);
}
