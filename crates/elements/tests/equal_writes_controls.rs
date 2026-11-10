//! `set_property_if_changed` on controls (IU2): it compares with the
//! property as the person's edits left it, so a binding written back to
//! a value the person has typed or toggled over still resets the
//! control, and a value equal to what the control shows leaves its
//! focus, caret, selection and undo alone.

mod support;

use support::{checkbox_page, number_page, text_page};
use vapors_dom::{Dom, PropertyValue};

fn text(value: &str) -> PropertyValue {
    PropertyValue::Text(value.to_owned())
}

/// The text page's first field bound to "Warm", focused with the caret
/// at its end.
fn bound_field() -> text_page::Page {
    let page = text_page::page();
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", text("Warm"));
    text_page::click(&page, [120.0, 12.0]);
    page
}

#[test]
fn a_field_typed_over_is_reset_when_the_binding_writes_its_old_value() {
    // Given
    let page = bound_field();
    text_page::type_text(&page, "th");

    // When
    let old = text("Warm");
    page.doc
        .borrow_mut()
        .set_property_if_changed(page.name, "value", old);

    // Then
    assert_eq!(text_page::value(&page, page.name), "Warm");
}

#[test]
fn the_value_a_field_shows_leaves_its_focus_selection_and_undo_alone() {
    // Given: "th" typed, "h" selected
    let page = bound_field();
    text_page::type_text(&page, "th");
    let (key, shift) = text_page::shift("ArrowLeft");
    text_page::key_with(&page, key, shift);
    page.doc.borrow_mut().take_changed();

    // When
    let shown = text("Warmth");
    page.doc
        .borrow_mut()
        .set_property_if_changed(page.name, "value", shown);

    // Then: unchanged, and undo still takes back the typing
    assert!(!page.doc.borrow_mut().take_changed());
    assert_eq!(page.doc.borrow().active_element(), Some(page.name));
    assert_eq!(text_page::selection(&page, page.name), (5, 6));
    text_page::key_with(&page, "z", text_page::control());
    assert_eq!(text_page::value(&page, page.name), "Warm");
}

#[test]
fn a_checkbox_toggled_by_the_person_is_reset_when_the_binding_writes_its_old_state() {
    // Given: bound unchecked, then clicked on
    let page = checkbox_page::page();
    let unchecked = PropertyValue::Bool(false);
    page.doc
        .borrow_mut()
        .set_property(page.visible, "checked", unchecked.clone());
    checkbox_page::click(&page, [5.0, 5.0]);
    assert!(checkbox_page::checked(&page, page.visible));

    // When
    page.doc
        .borrow_mut()
        .set_property_if_changed(page.visible, "checked", unchecked);

    // Then
    assert!(!checkbox_page::checked(&page, page.visible));
}

#[test]
fn an_out_of_range_number_typed_over_a_binding_is_reset_and_valid_again() {
    // Given: bound to 20, then "200" typed over it (max 100)
    let page = number_page::page();
    let twenty = PropertyValue::Number(20.0);
    page.doc
        .borrow_mut()
        .set_property(page.amount, "value", twenty.clone());
    number_page::key(&page, "Tab");
    number_page::type_text(&page, "200");
    assert!(invalid(&page));

    // When
    page.doc
        .borrow_mut()
        .set_property_if_changed(page.amount, "value", twenty);

    // Then
    assert_eq!(number_page::value(&page, page.amount), "20");
    assert!(!invalid(&page));
}

/// Whether `amount` is painted by an `input:invalid` rule.
fn invalid(page: &number_page::Page) -> bool {
    let mut doc = page.doc.borrow_mut();
    let rule = "input:invalid { background-color: #ff0000; }";
    doc.add_stylesheet("invalid.css", rule).expect("parses");
    let painted = doc
        .update(support::viewport(400.0, 100.0))
        .expect("updates");
    let red = vapors_scene::Color::from_rgba8(255, 0, 0, 255);
    painted.scene().items().iter().any(|item| {
        matches!(item, vapors_scene::Item::Quad(q) if q.rect.top == 0.0 && q.background == red)
    })
}
