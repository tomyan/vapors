//! What a `StyleKey` refuses in `RecordingDom`: a key whose shape a
//! tree cannot keep true, `set_attribute` of an attribute the key
//! fixes (whatever its case, and through `set_attribute_if_changed`
//! even when the value is equal) and a toggle the key does not list.
//! Each is recorded as `Call::Refused`, naming the site and the fix,
//! and changes nothing.

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, NodeId, SCOPE_ATTRIBUTE, StyleKey};

static ROW: StyleKey = StyleKey {
    site: "row.wisp:14:5",
    scope: Some("row.wisp"),
    tag: "div",
    classes: &["row"],
    toggles: &["marked", "wide"],
    style: Some("height: 24px"),
};

static PLAIN: StyleKey = StyleKey {
    site: "plain.wisp:3:1",
    scope: None,
    tag: "span",
    classes: &[],
    toggles: &[],
    style: None,
};

/// The refusals recorded on `node`.
fn refusals(dom: &RecordingDom, node: NodeId) -> Vec<String> {
    (dom.calls().iter())
        .filter_map(|call| match call {
            Call::Refused { node: on, refusal } if *on == node => Some(refusal.clone()),
            _ => None,
        })
        .collect()
}

fn keyed(key: &'static StyleKey) -> (RecordingDom, NodeId) {
    let mut dom = RecordingDom::new();
    let node = dom.create_keyed_element(key);
    (dom, node)
}

#[test]
fn setting_an_attribute_the_key_fixes_is_refused_in_any_case_and_changes_nothing() {
    let class_fix = "toggle a class with `class:name={bool}` there";
    for (name, fix) in [
        ("class", class_fix),
        ("Class", class_fix),
        ("style", "write it there"),
        ("STYLE", "write it there"),
        (SCOPE_ATTRIBUTE, "write it there"),
    ] {
        // Given
        let (mut dom, div) = keyed(&ROW);
        let before = dom.attribute(div, &name.to_ascii_lowercase());

        // When
        dom.set_attribute(div, name, "other");

        // Then
        let expected = format!(
            "set_attribute: `{name}` on <div> at row.wisp:14:5 is set from its template; {fix}"
        );
        assert_eq!(refusals(&dom, div), vec![expected], "{name}");
        assert_eq!(
            dom.attribute(div, &name.to_ascii_lowercase()),
            before,
            "{name}"
        );
        assert_eq!(dom.attribute(div, "Class"), None, "{name}");
    }
}

#[test]
fn set_attribute_if_changed_refuses_a_fixed_attribute_even_with_its_current_value() {
    // Given
    let (mut dom, div) = keyed(&ROW);

    // When
    dom.set_attribute_if_changed(div, "class", "row");

    // Then
    assert_eq!(refusals(&dom, div).len(), 1);
}

#[test]
fn other_attributes_of_a_keyed_element_are_not_refused() {
    // Given
    let (mut dom, div) = keyed(&ROW);

    // When
    dom.set_attribute(div, "id", "first");
    dom.set_attribute_if_changed(div, "title", "a row");

    // Then
    assert_eq!(refusals(&dom, div), Vec::<String>::new());
    assert_eq!(dom.attribute(div, "id").as_deref(), Some("first"));
}

#[test]
fn toggling_a_name_the_key_does_not_list_is_refused_naming_the_directives() {
    // Given
    let (mut dom, div) = keyed(&ROW);

    // When
    dom.toggle_classes(div, &[("dim", true)]);

    // Then
    assert_eq!(
        refusals(&dom, div),
        vec![
            "toggle_classes: `class:dim` is not a directive of <div> at row.wisp:14:5, whose directives are `marked`, `wide`; add `class:dim={bool}` there"
        ]
    );
    assert_eq!(dom.attribute(div, "class").as_deref(), Some("row"));
}

#[test]
fn toggling_on_a_key_without_directives_says_it_has_none() {
    // Given
    let (mut dom, span) = keyed(&PLAIN);

    // When
    dom.toggle_classes(span, &[("dim", true)]);

    // Then
    assert!(refusals(&dom, span)[0].contains(
        "at plain.wisp:3:1, which has no `class:` directives; add `class:dim={bool}` there"
    ));
}

#[test]
fn toggling_on_an_unkeyed_element_is_refused_naming_the_fix() {
    // Given
    let mut dom = RecordingDom::new();
    let div = dom.create_element("div");

    // When
    dom.toggle_classes(div, &[("marked", true)]);

    // Then
    assert!(refusals(&dom, div)[0].ends_with(
        "was not created by create_keyed_element, so it has no `class:` directives; create it with create_keyed_element from its template's StyleKey, or set `class` with set_attribute"
    ));
}
