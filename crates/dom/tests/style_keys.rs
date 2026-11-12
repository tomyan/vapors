//! A template element's `StyleKey`: `create_keyed_element` sets the
//! element's scope attribute, `class` and `style` from it, and
//! `toggle_classes` turns its `class:` directives on and off. What it
//! refuses: `style_key_refusals.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, SCOPE_ATTRIBUTE, StyleKey};

static ROW: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some("vapors-perf/ui/mount_row.wisp"),
    tag: "div",
    classes: &["row"],
    toggles: &["marked", "wide"],
    style: Some("height: 24px"),
};

static PLAIN: StyleKey = StyleKey {
    site: "src/plain.wisp:3",
    scope: None,
    tag: "span",
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
    assert_eq!(dom.attribute(div, "style").as_deref(), Some("height: 24px"));
}

#[test]
fn a_key_without_scope_classes_or_style_sets_no_attribute() {
    // Given
    let dom = dom();

    // When
    let span = dom.borrow_mut().create_keyed_element(&PLAIN);

    // Then
    let dom = dom.borrow();
    for name in [SCOPE_ATTRIBUTE, "class", "style"] {
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

static ROW_AGAIN: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some("vapors-perf/ui/mount_row.wisp"),
    tag: "div",
    classes: &["row"],
    toggles: &["marked", "wide"],
    style: Some("height: 24px"),
};

#[test]
fn keys_compare_by_address_so_equal_content_is_two_keys() {
    // Then
    assert_eq!(&ROW, &ROW);
    assert_ne!(&ROW, &ROW_AGAIN);
}
