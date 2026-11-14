//! What a keyed element refuses in the document: `set_attribute` of an
//! attribute its key fixes (whatever its case, and through
//! `set_attribute_if_changed` even when the value is equal), a toggle
//! its key does not list, a toggle on an element without a key, and a
//! malformed key. Each refused call changes nothing and is returned
//! from every update while the element is in the tree, naming the
//! site and the fix; none panics.

mod support;

use support::{document, viewport};
use vapors_dom::{Dom, NodeId, SCOPE_ATTRIBUTE, StyleKey};
use vapors_elements::{Document, ElementError};

static ROW: StyleKey = StyleKey {
    site: "row.wisp:14:5",
    scope: Some("row.wisp"),
    tag: "div",
    classes: &["row"],
    toggles: &["marked"],
    attributes: &[("title", "a row")],
    style: Some("width: 50px"),
};

static TWICE: StyleKey = StyleKey {
    site: "row.wisp:20:1",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &["on", "on"],
    attributes: &[],
    style: None,
};

/// A document with an element made from `key` under the body.
fn page(key: &'static StyleKey) -> (Document, NodeId) {
    let mut doc = document();
    let node = doc.create_keyed_element(key);
    let body = doc.body();
    doc.append(body, node);
    (doc, node)
}

/// The update's refusals, as text.
fn refusals(doc: &mut Document) -> Vec<String> {
    match doc.update(viewport(200.0, 100.0)) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn setting_an_attribute_the_key_fixes_is_refused_in_any_case_and_changes_nothing() {
    let class_fix = "toggle a class with `class:name={bool}` there";
    for (name, fix) in [
        ("class", class_fix),
        ("CLASS", class_fix),
        ("style", "write it there"),
        ("Style", "write it there"),
        (SCOPE_ATTRIBUTE, "write it there"),
        ("title", "write `title={expr}` there"),
    ] {
        // Given
        let (mut doc, row) = page(&ROW);
        let before = doc
            .attribute(row, &name.to_ascii_lowercase())
            .map(str::to_owned);

        // When
        doc.set_attribute(row, name, "other");

        // Then
        let expected = format!(
            "set_attribute: `{name}` on <div> at row.wisp:14:5 is set from its template; {fix}"
        );
        assert_eq!(refusals(&mut doc), vec![expected], "{name}");
        let after = doc
            .attribute(row, &name.to_ascii_lowercase())
            .map(str::to_owned);
        assert_eq!(after, before, "{name}");
    }
}

#[test]
fn a_refused_call_is_returned_from_every_update_while_the_element_is_in_the_tree() {
    // Given
    let (mut doc, row) = page(&ROW);
    doc.set_attribute(row, "class", "other");
    assert_eq!(refusals(&mut doc).len(), 1);

    // When
    let again = refusals(&mut doc);
    doc.remove(row);

    // Then
    assert_eq!(again.len(), 1);
    assert_eq!(refusals(&mut doc), Vec::<String>::new());
}

#[test]
fn set_attribute_if_changed_refuses_a_fixed_attribute_even_with_its_current_value() {
    // Given
    let (mut doc, row) = page(&ROW);

    // When
    doc.set_attribute_if_changed(row, "class", "row");

    // Then
    assert!(matches!(
        doc.update(viewport(200.0, 100.0)),
        Err(errors) if matches!(errors.as_slice(), [ElementError::Refused(_)])
    ));
}

#[test]
fn toggling_a_name_the_key_does_not_list_is_refused() {
    // Given
    let (mut doc, row) = page(&ROW);

    // When
    doc.toggle_classes(row, &[("dim", true)]);

    // Then
    assert_eq!(
        refusals(&mut doc),
        vec![
            "toggle_classes: `class:dim` is not a directive of <div> at row.wisp:14:5, whose directives are `marked`; add `class:dim={bool}` there"
        ]
    );
}

#[test]
fn toggling_on_an_unkeyed_element_is_refused_naming_the_fix() {
    // Given
    let mut doc = document();
    let div = doc.create_element("div");
    let body = doc.body();
    doc.append(body, div);

    // When
    doc.toggle_classes(div, &[("marked", true)]);

    // Then
    assert_eq!(
        refusals(&mut doc),
        vec![
            "toggle_classes: <div> was not created by create_keyed_element, so it has no `class:` directives; create it with create_keyed_element from its template's StyleKey, or set `class` with set_attribute"
        ]
    );
}

#[test]
fn an_element_from_a_malformed_key_is_refused_once_it_is_in_the_tree() {
    // Given
    let mut doc = document();
    let div = doc.create_keyed_element(&TWICE);
    assert_eq!(refusals(&mut doc), Vec::<String>::new());

    // When
    let body = doc.body();
    doc.append(body, div);

    // Then
    assert_eq!(
        refusals(&mut doc),
        vec![
            "create_keyed_element: the StyleKey of <div> at row.wisp:20:1 lists `class:on` twice; give each directive once"
        ]
    );
}
