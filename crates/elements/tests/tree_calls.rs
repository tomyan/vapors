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

#[test]
#[should_panic(expected = "the document's <body>")]
fn removing_the_body_panics_naming_it() {
    // Given
    let mut doc = document();
    let body = doc.body();

    // When
    doc.remove(body);
}

#[test]
#[should_panic(expected = "the document's <html>")]
fn removing_the_root_panics_naming_it() {
    // Given
    let mut doc = document();
    let root = doc.root();

    // When
    doc.remove(root);
}

#[test]
fn each_change_to_what_is_shown_asks_for_a_frame_and_listening_does_not() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "");
    let words = text(&mut doc, div, "a");
    let changes: [(&str, Box<dyn Fn(&mut vapors_elements::Document)>); 6] = [
        ("set_text", Box::new(move |d| d.set_text(words, "b"))),
        (
            "set_attribute",
            Box::new(move |d| d.set_attribute(div, "class", "x")),
        ),
        (
            "set_property",
            Box::new(move |d| d.set_property(div, "value", vapors_dom::PropertyValue::Bool(true))),
        ),
        (
            "append",
            Box::new(move |d| {
                let n = d.create_element("div");
                d.append(div, n);
            }),
        ),
        (
            "insert_before",
            Box::new(move |d| {
                let n = d.create_text("c");
                d.insert_before(div, n, Some(words));
            }),
        ),
        ("remove", Box::new(move |d| d.remove(words))),
    ];

    for (name, change) in changes {
        doc.take_changed();

        // When
        change(&mut doc);

        // Then
        assert!(doc.take_changed(), "{name} asks for a frame");
    }
    doc.listen(div, "click", Box::new(|_: &mut vapors_dom::Event| {}));
    assert!(!doc.take_changed(), "a listener changes nothing shown");
}
