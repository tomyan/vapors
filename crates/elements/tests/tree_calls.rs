//! The generated code's calls on a Document: they build the tree, and a
//! call the tree cannot honour panics naming the call.

mod support;

use support::{document, element, text};
use vapors_dom::Dom;

#[test]
fn append_and_insert_before_order_children() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let b = element(&mut doc, body, "div", "b");
    let a = doc.create_element("div");

    // When
    doc.insert_before(body, a, Some(b));
    let c = element(&mut doc, body, "div", "c");

    // Then
    assert_eq!(doc.children(body), [a, b, c]);
    assert_eq!(doc.parent(a), Some(body));
}

#[test]
fn inserting_a_placed_node_moves_it() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let a = element(&mut doc, body, "div", "");
    let b = element(&mut doc, body, "div", "");
    let c = element(&mut doc, body, "div", "");

    // When
    doc.insert_before(body, c, Some(a));

    // Then
    assert_eq!(doc.children(body), [c, a, b]);
}

#[test]
fn inserting_a_node_before_itself_leaves_it_in_place() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let a = element(&mut doc, body, "div", "");
    let b = element(&mut doc, body, "div", "");

    // When
    doc.insert_before(body, b, Some(b));

    // Then
    assert_eq!(doc.children(body), [a, b]);
}

#[test]
fn set_text_replaces_a_text_nodes_text() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let node = text(&mut doc, body, "before");

    // When
    doc.set_text(node, "after");

    // Then
    assert_eq!(doc.text(node), Some("after"));
}

#[test]
fn remove_detaches_the_node_and_frees_its_subtree() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "");
    let inner = text(&mut doc, outer, "x");

    // When
    doc.remove(outer);

    // Then
    assert_eq!(doc.children(body), []);
    assert!(!doc.contains(outer) && !doc.contains(inner));
}

#[test]
#[should_panic(expected = "set_text: node")]
fn a_call_with_a_removed_node_panics_naming_the_call() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let node = text(&mut doc, body, "x");
    doc.remove(node);

    // When
    doc.set_text(node, "y");
}

#[test]
#[should_panic(expected = "only elements hold children")]
fn appending_to_a_text_node_panics() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let parent = text(&mut doc, body, "x");
    let child = doc.create_element("div");

    // When
    doc.append(parent, child);
}

#[test]
#[should_panic(expected = "would make a cycle")]
fn placing_a_node_inside_its_own_descendant_panics() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "");
    let inner = element(&mut doc, outer, "div", "");

    // When
    doc.append(inner, outer);
}

#[test]
#[should_panic(expected = "`before`")]
fn inserting_before_a_node_of_another_parent_panics() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let a = element(&mut doc, body, "div", "");
    let elsewhere = element(&mut doc, a, "div", "");
    let child = doc.create_element("div");

    // When
    doc.insert_before(body, child, Some(elsewhere));
}

#[test]
#[should_panic(expected = "is a text node")]
fn setting_an_attribute_on_a_text_node_panics() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let node = text(&mut doc, body, "x");

    // When
    doc.set_attribute(node, "class", "a");
}
