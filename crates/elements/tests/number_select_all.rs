//! Control+A in a focused number field selects its text whole, as it
//! does in a text field and as Chromium does: typing then replaces the
//! text and Backspace clears it, and the field uses the key, so an app
//! does not also act on it. The order against Chromium is pinned by the
//! corpus's `number.events`.

mod support;

use support::number_page::{click, key, page, type_text, value};
use vapors_elements::{Document, Modifiers};

fn control_a(page: &support::number_page::Page) -> bool {
    let control = Modifiers {
        control: true,
        ..Modifiers::default()
    };
    let used = Document::key_pressed(&page.doc, "a", control);
    Document::key_released(&page.doc, "a", control);
    used
}

#[test]
fn control_a_selects_the_text_whole_so_typing_replaces_it() {
    // Given: focused by a click, which selects nothing
    let page = page();
    click(&page, [50.0, 12.0]);

    // When
    let used = control_a(&page);
    let selected = page.doc.borrow().text_selected(page.amount);
    type_text(&page, "7");

    // Then
    assert!(used, "the field uses Control+A");
    assert!(selected);
    assert_eq!(value(&page, page.amount), "7");
    assert_eq!(*page.log.borrow(), ["amount input \"7\" 7"]);
}

#[test]
fn control_a_then_backspace_clears_the_text() {
    // Given
    let page = page();
    click(&page, [50.0, 12.0]);

    // When
    control_a(&page);
    key(&page, "Backspace");

    // Then
    assert_eq!(value(&page, page.amount), "");
    assert_eq!(*page.log.borrow(), ["amount input \"\" none"]);
}

/// Guards that select-all is a focused field's key, not the document's:
/// with no field focused the app gets Control+A.
#[test]
fn control_a_on_a_field_without_focus_selects_nothing() {
    // Given: nothing focused
    let page = page();

    // When
    let used = control_a(&page);

    // Then
    assert!(!used, "with no field focused, Control+A is the app's");
    assert!(!page.doc.borrow().text_selected(page.amount));
}
