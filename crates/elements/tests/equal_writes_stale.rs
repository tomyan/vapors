//! The `current_*` reads the `_if_changed` setters make refuse a removed
//! node by naming the read, as the other calls do.

mod support;

use support::{document, element, text};
use vapors_dom::Dom;

#[test]
#[should_panic(expected = "current_text: node")]
fn current_text_of_a_removed_node_panics_naming_the_call() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let node = text(&mut doc, body, "x");
    doc.remove(node);

    // When
    let _ = doc.current_text(node);
}

#[test]
#[should_panic(expected = "current_attribute: node")]
fn current_attribute_of_a_removed_node_panics_naming_the_call() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let node = element(&mut doc, body, "div", "x");
    doc.remove(node);

    // When
    let _ = doc.current_attribute(node, "class");
}
