//! The style phase's checks are kept between updates and redone for the
//! elements changed since (IU3 PR 3a): each change that can add or drop
//! a refusal is seen by the next update.

mod support;

use support::{document, element, refusals_at, viewport};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

/// A document with a div, updated once.
fn updated() -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    (doc, div)
}

fn listen_unfired(doc: &mut Document, node: vapors_dom::NodeId) {
    doc.listen(node, "optionchoose", Box::new(|_: &mut Event| {}));
}

#[test]
fn a_listener_added_after_an_update_for_an_event_never_fired_is_refused() {
    // Given
    let (mut doc, div) = updated();

    // When
    listen_unfired(&mut doc, div);

    // Then
    let found = refusals_at(&mut doc, viewport(100.0, 100.0));
    assert!(
        found.iter().any(|r| r.contains("onoptionchoose")),
        "{found:?}"
    );
}

#[test]
fn declaring_the_event_after_a_refusal_lets_the_update_through() {
    // Given
    let (mut doc, div) = updated();
    listen_unfired(&mut doc, div);
    refusals_at(&mut doc, viewport(100.0, 100.0));

    // When
    doc.declare_events(&["optionchoose"]);

    // Then
    assert!(doc.update(viewport(100.0, 100.0)).is_ok());
}

#[test]
fn removing_a_refused_element_lets_the_update_through() {
    // Given
    let (mut doc, _) = updated();
    let body = doc.body();
    let canvas = element(&mut doc, body, "canvas", "");
    refusals_at(&mut doc, viewport(100.0, 100.0));

    // When
    doc.remove(canvas);

    // Then
    assert!(doc.update(viewport(100.0, 100.0)).is_ok());
}

#[test]
fn an_attribute_set_after_an_update_that_the_tree_refuses_is_refused() {
    // Given
    let (mut doc, div) = updated();

    // When
    doc.set_attribute(div, "tabindex", "2");

    // Then
    let found = refusals_at(&mut doc, viewport(100.0, 100.0));
    assert!(found.iter().any(|r| r.contains("tabindex")), "{found:?}");
}
