//! A keyed element in the document: its attributes come from its
//! `StyleKey`, its `class:` directives toggle through `toggle_classes`
//! and restyle it, and the attributes the key fixes are refused to
//! `set_attribute`, naming the template site.

mod support;

use support::{document, viewport};
use vapors_dom::{Dom, NodeId, SCOPE_ATTRIBUTE, StyleKey};
use vapors_elements::Document;

const SCOPE: &str = "vapors-perf/ui/mount_row.wisp";

static ROW: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some(SCOPE),
    tag: "div",
    classes: &["row"],
    toggles: &["marked"],
    style: Some("width: 50px"),
};

/// A document with the row's sheet under its scope and one keyed row
/// under the body.
fn row_page() -> (Document, NodeId) {
    let mut doc = document();
    doc.add_style(
        SCOPE,
        ".row { color: rgb(1 2 3); } .row.marked { color: rgb(4 5 6); }",
    );
    let row = doc.create_keyed_element(&ROW);
    let body = doc.body();
    doc.append(body, row);
    (doc, row)
}

fn color(doc: &mut Document, node: NodeId) -> String {
    doc.update(viewport(200.0, 100.0)).expect("updates");
    doc.computed_style(node)
        .expect("styled")
        .get_by_name("color")
        .expect("a longhand")
        .to_css()
}

fn width(doc: &mut Document, node: NodeId) -> String {
    doc.update(viewport(200.0, 100.0)).expect("updates");
    doc.computed_style(node)
        .expect("styled")
        .get_by_name("width")
        .expect("a longhand")
        .to_css()
}

#[test]
fn a_keyed_element_is_styled_by_its_scope_classes_and_inline_style() {
    // Given
    let (mut doc, row) = row_page();

    // Then
    assert_eq!(doc.attribute(row, SCOPE_ATTRIBUTE), Some(SCOPE));
    assert_eq!(color(&mut doc, row), "rgb(1 2 3)");
    assert_eq!(width(&mut doc, row), "50px");
}

#[test]
fn a_toggled_class_restyles_the_element_and_toggling_off_restores_it() {
    // Given
    let (mut doc, row) = row_page();
    assert_eq!(color(&mut doc, row), "rgb(1 2 3)");

    // When
    doc.toggle_classes(row, &[("marked", true)]);

    // Then
    assert_eq!(doc.attribute(row, "class"), Some("row marked"));
    assert_eq!(color(&mut doc, row), "rgb(4 5 6)");

    // When
    doc.toggle_classes(row, &[("marked", false)]);

    // Then
    assert_eq!(color(&mut doc, row), "rgb(1 2 3)");
}

#[test]
fn toggling_to_the_same_state_restyles_nothing() {
    // Given
    let (mut doc, row) = row_page();
    doc.toggle_classes(row, &[("marked", true)]);
    let _ = color(&mut doc, row);

    // When
    doc.toggle_classes(row, &[("marked", true)]);
    doc.update(viewport(200.0, 100.0)).expect("updates");

    // Then
    assert_eq!(doc.elements_restyled(), 0);
}

#[test]
#[should_panic(
    expected = "`class` on <div> at crates/perf/ui/mount_row.wisp:14 is set from its template; toggle a class with `class:name={bool}` there"
)]
fn setting_class_on_a_keyed_element_is_refused_naming_the_directive() {
    let (mut doc, row) = row_page();
    doc.set_attribute(row, "class", "other");
}

#[test]
#[should_panic(
    expected = "`style` on <div> at crates/perf/ui/mount_row.wisp:14 is set from its template; write it there"
)]
fn setting_style_on_a_keyed_element_is_refused() {
    let (mut doc, row) = row_page();
    doc.set_attribute(row, "style", "width: 10px");
}

#[test]
#[should_panic(
    expected = "`class:dim` is not a directive of <div> at crates/perf/ui/mount_row.wisp:14"
)]
fn toggling_a_name_the_key_does_not_list_is_refused() {
    let (mut doc, row) = row_page();
    doc.toggle_classes(row, &[("dim", true)]);
}

#[test]
fn other_attributes_of_a_keyed_element_may_be_set() {
    // Given
    let (mut doc, row) = row_page();

    // When
    doc.set_attribute(row, "title", "a row");

    // Then
    assert_eq!(doc.attribute(row, "title"), Some("a row"));
}
