//! Select-all is A with Command on macOS and with Control elsewhere, in
//! a text field and a number field alike, as Chromium has it: on macOS
//! Control+A does not select all (Chromium moves to the line's start),
//! and elsewhere Meta+A is not select-all. The app says which platform
//! it runs on (`Document::set_shortcut_modifier`); until it does, the
//! target OS's (`text_field.rs` and `number_select_all.rs` pin Control).

mod support;

use support::{number_page, text_page};
use vapors_dom::{Dom, PropertyValue};
use vapors_elements::{Modifiers, ShortcutModifier};

fn modifier_for(mac: bool) -> ShortcutModifier {
    if mac {
        ShortcutModifier::Command
    } else {
        ShortcutModifier::Control
    }
}

fn command() -> Modifiers {
    Modifiers {
        meta: true,
        ..Modifiers::default()
    }
}

/// The text page's field holding "Warm day", focused by a click with
/// the caret at the end, on macOS when `mac`.
fn text_field(mac: bool) -> text_page::Page {
    let page = text_page::page();
    page.doc
        .borrow_mut()
        .set_shortcut_modifier(modifier_for(mac));
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", PropertyValue::Text("Warm day".into()));
    text_page::click(&page, [150.0, 12.0]);
    page
}

/// The number page's field, focused by a click, on macOS when `mac`.
fn number_field(mac: bool) -> number_page::Page {
    let page = number_page::page();
    page.doc
        .borrow_mut()
        .set_shortcut_modifier(modifier_for(mac));
    number_page::click(&page, [50.0, 12.0]);
    page
}

fn number_key(page: &number_page::Page, modifiers: Modifiers) {
    vapors_elements::Document::key_pressed(&page.doc, "a", modifiers);
    vapors_elements::Document::key_released(&page.doc, "a", modifiers);
}

#[test]
fn an_unset_document_takes_the_target_oss_modifier() {
    // Given
    let page = number_page::page();

    // When
    let modifier = page.doc.borrow().shortcut_modifier();

    // Then: natively the target decides, not a fixed Control
    assert_eq!(modifier, modifier_for(cfg!(target_os = "macos")));
}

#[test]
fn on_macos_command_a_selects_a_text_fields_text() {
    // Given
    let page = text_field(true);

    // When
    text_page::key_with(&page, "a", command());

    // Then
    assert_eq!(text_page::selection(&page, page.name), (0, 8));
}

#[test]
fn on_macos_control_a_does_not_select_a_text_fields_text() {
    // Given
    let page = text_field(true);

    // When
    text_page::key_with(&page, "a", text_page::control());

    // Then
    assert_eq!(text_page::selection(&page, page.name), (8, 8));
}

#[test]
fn elsewhere_meta_a_does_not_select_a_text_fields_text() {
    // Given
    let page = text_field(false);

    // When
    text_page::key_with(&page, "a", command());

    // Then
    assert_eq!(text_page::selection(&page, page.name), (8, 8));
}

#[test]
fn on_macos_command_a_selects_a_number_fields_text() {
    // Given
    let page = number_field(true);

    // When
    number_key(&page, command());

    // Then
    assert!(page.doc.borrow().text_selected(page.amount));
}

#[test]
fn on_macos_control_a_does_not_select_a_number_fields_text() {
    // Given
    let page = number_field(true);

    // When
    number_key(&page, text_page::control());

    // Then
    assert!(!page.doc.borrow().text_selected(page.amount));
}

#[test]
fn elsewhere_meta_a_does_not_select_a_number_fields_text() {
    // Given
    let page = number_field(false);

    // When
    number_key(&page, command());

    // Then
    assert!(!page.doc.borrow().text_selected(page.amount));
}
