//! A template element's `StyleKey`: `create_keyed_element` sets the
//! element's scope attribute, `class`, `id` and `style` from it,
//! `toggle_classes` turns its `class:` directives on and off, and the
//! attributes the key fixes are refused to `set_attribute`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, SCOPE_ATTRIBUTE, StyleKey};

static ROW: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some("vapors-perf/ui/mount_row.wisp"),
    tag: "div",
    id: Some("first"),
    classes: &["row"],
    toggles: &["marked", "wide"],
    style: Some("height: 24px"),
};

static PLAIN: StyleKey = StyleKey {
    site: "src/plain.wisp:3",
    scope: None,
    tag: "span",
    id: None,
    classes: &[],
    toggles: &[],
    style: None,
};

fn dom() -> Rc<RefCell<RecordingDom>> {
    Rc::new(RefCell::new(RecordingDom::new()))
}

#[test]
fn a_keyed_element_carries_the_keys_attributes() {
    // Given
    let dom = dom();
    let root = dom.borrow().root();

    // When
    let div = dom.borrow_mut().create_keyed_element(&ROW);
    dom.borrow_mut().append(root, div);

    // Then
    assert_eq!(
        dom.borrow().calls()[0],
        Call::CreateKeyedElement {
            node: div,
            key: &ROW
        }
    );
    let dom = dom.borrow();
    assert_eq!(
        dom.attribute(div, SCOPE_ATTRIBUTE).as_deref(),
        Some("vapors-perf/ui/mount_row.wisp")
    );
    assert_eq!(dom.attribute(div, "class").as_deref(), Some("row"));
    assert_eq!(dom.attribute(div, "id").as_deref(), Some("first"));
    assert_eq!(dom.attribute(div, "style").as_deref(), Some("height: 24px"));
}

#[test]
fn a_key_without_scope_classes_id_or_style_sets_no_attribute() {
    // Given
    let dom = dom();

    // When
    let span = dom.borrow_mut().create_keyed_element(&PLAIN);

    // Then
    let dom = dom.borrow();
    for name in [SCOPE_ATTRIBUTE, "class", "id", "style"] {
        assert_eq!(dom.attribute(span, name), None, "{name}");
    }
}

#[test]
fn toggling_adds_each_directive_on_after_the_literal_names_in_key_order() {
    // Given
    let dom = dom();
    let div = dom.borrow_mut().create_keyed_element(&ROW);
    dom.borrow_mut().clear_calls();

    // When
    dom.borrow_mut().toggle_classes(div, &[("wide", true)]);
    dom.borrow_mut().toggle_classes(div, &[("marked", true)]);

    // Then
    assert_eq!(
        dom.borrow().attribute(div, "class").as_deref(),
        Some("row marked wide")
    );
    assert_eq!(
        dom.borrow().calls()[0],
        Call::ToggleClasses {
            node: div,
            toggles: vec![("wide".into(), true)]
        }
    );
}

#[test]
fn toggling_off_removes_the_name_and_leaves_the_literal_names() {
    // Given
    let dom = dom();
    let div = dom.borrow_mut().create_keyed_element(&ROW);
    dom.borrow_mut()
        .toggle_classes(div, &[("marked", true), ("wide", true)]);

    // When
    dom.borrow_mut().toggle_classes(div, &[("marked", false)]);

    // Then
    assert_eq!(
        dom.borrow().attribute(div, "class").as_deref(),
        Some("row wide")
    );
}

#[test]
#[should_panic(
    expected = "`class` on <div> at crates/perf/ui/mount_row.wisp:14 is set from its template; toggle a class with `class:name={bool}` there"
)]
fn setting_class_on_a_keyed_element_is_refused_naming_the_directive() {
    let dom = dom();
    let div = dom.borrow_mut().create_keyed_element(&ROW);
    dom.borrow_mut().set_attribute(div, "class", "other");
}

#[test]
#[should_panic(
    expected = "`id` on <div> at crates/perf/ui/mount_row.wisp:14 is set from its template; write it there"
)]
fn setting_id_on_a_keyed_element_is_refused() {
    let dom = dom();
    let div = dom.borrow_mut().create_keyed_element(&ROW);
    dom.borrow_mut().set_attribute(div, "id", "other");
}

#[test]
#[should_panic(expected = "`data-wisp` on <span> at src/plain.wisp:3 is set from its template")]
fn setting_the_scope_attribute_on_a_keyed_element_is_refused() {
    let dom = dom();
    let span = dom.borrow_mut().create_keyed_element(&PLAIN);
    dom.borrow_mut().set_attribute(span, SCOPE_ATTRIBUTE, "x");
}

#[test]
#[should_panic(
    expected = "`class:dim` is not a directive of <div> at crates/perf/ui/mount_row.wisp:14, whose directives are `marked`, `wide`; add `class:dim={bool}` there"
)]
fn toggling_a_name_the_key_does_not_list_is_refused() {
    let dom = dom();
    let div = dom.borrow_mut().create_keyed_element(&ROW);
    dom.borrow_mut().toggle_classes(div, &[("dim", true)]);
}

#[test]
#[should_panic(
    expected = "was not created by create_keyed_element, so it has no `class:` directives"
)]
fn toggling_on_an_unkeyed_element_is_refused() {
    let dom = dom();
    let div = dom.borrow_mut().create_element("div");
    dom.borrow_mut().toggle_classes(div, &[("marked", true)]);
}

#[test]
fn other_attributes_of_a_keyed_element_may_be_set() {
    // Given
    let dom = dom();
    let div = dom.borrow_mut().create_keyed_element(&ROW);

    // When
    dom.borrow_mut().set_attribute(div, "title", "a row");

    // Then
    assert_eq!(
        dom.borrow().attribute(div, "title").as_deref(),
        Some("a row")
    );
}
