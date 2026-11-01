//! A text field (E6a): typing replaces the selection, Control moves and
//! deletes by word, Control+A selects all, a drag selects, a disabled
//! field takes the caret but no text, a signal's text reaches the
//! editor and the editor's text the property, the selection counts in
//! UTF-16 units, and `selectionchange` fires once per settle. The order
//! against Chromium is pinned by the corpus's `text.events`.

mod support;

use support::text_page::{
    click, control, key, key_with, page, selection, settle, shift, type_text, value,
};
use vapors_dom::{Dom, PropertyValue};
use vapors_elements::Document;

#[test]
fn typing_replaces_a_selection_made_with_shift_and_the_arrows() {
    // Given: the caret at the end by a click right of the text
    let page = page();
    click(&page, [120.0, 12.0]);

    // When: two back with Shift, then typed
    let (k, m) = shift("ArrowLeft");
    key_with(&page, k, m);
    key_with(&page, k, m);
    type_text(&page, "p");

    // Then: "Wap", caret after the p
    assert_eq!(value(&page, page.name), "Wap");
    assert_eq!(selection(&page, page.name), (3, 3));
}

#[test]
fn control_moves_and_deletes_by_word_and_control_a_selects_all() {
    // Given: "Warm day", the caret at the end
    let page = page();
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", PropertyValue::Text("Warm day".into()));
    click(&page, [150.0, 12.0]);

    // When
    key_with(&page, "ArrowLeft", control());
    let before_day = selection(&page, page.name);
    key_with(&page, "Backspace", control());
    let after = value(&page, page.name);
    key_with(&page, "a", control());

    // Then
    assert_eq!(before_day, (5, 5));
    assert_eq!(after, "day");
    assert_eq!(selection(&page, page.name), (0, 3));
}

#[test]
fn a_drag_selects_from_the_press_to_the_pointer() {
    // Given: "Warm" from x 3, about 9 px a letter
    let page = page();

    // When: pressed before W, dragged past r
    Document::pointer_pressed_clicks(&page.doc, [4.0, 12.0], std::num::NonZeroU32::MIN);
    Document::pointer_moved(&page.doc, [30.0, 12.0]);
    Document::pointer_released(&page.doc, [30.0, 12.0]);

    // Then
    let (start, end) = selection(&page, page.name);
    assert_eq!(start, 0);
    assert!((2..=3).contains(&end), "{end}");
    assert_eq!(page.doc.borrow().active_element(), Some(page.name));
}

#[test]
fn a_disabled_field_takes_no_caret_no_focus_and_no_text() {
    // Given
    let page = page();

    // When
    click(&page, [150.0, 60.0]);
    type_text(&page, "x");

    // Then: the selection left at the start, as Chromium leaves it
    assert_eq!(selection(&page, page.off), (0, 0));
    assert_eq!(page.doc.borrow().active_element(), None);
    assert_eq!(value(&page, page.off), "off");
}

#[test]
fn a_signals_text_reaches_the_editor_and_typing_reaches_the_property() {
    // Given: focused, the caret at the end
    let page = page();
    click(&page, [120.0, 12.0]);

    // When: a signal sets the text, then a letter is typed
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", PropertyValue::Text("Hi".into()));
    let set = (value(&page, page.name), selection(&page, page.name));
    type_text(&page, "!");

    // Then: the caret clamped to the new text; the property follows the edit
    assert_eq!(set, ("Hi".to_owned(), (2, 2)));
    assert_eq!(value(&page, page.name), "Hi!");
    assert!(page.log.borrow().contains(&"input \"Hi!\"".to_owned()));
}

#[test]
fn the_selection_counts_utf16_units_as_the_dom_does() {
    // Given: a text with an astral character (two UTF-16 units, four bytes)
    let page = page();
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", PropertyValue::Text("a😀b".into()));

    // When: Tab focuses and selects all
    key(&page, "Tab");

    // Then
    assert_eq!(selection(&page, page.name), (0, 4));
}

#[test]
fn selectionchange_fires_once_per_settle_and_only_when_the_selection_moved() {
    // Given
    let page = page();
    click(&page, [120.0, 12.0]);
    settle(&page);
    page.log.borrow_mut().clear();

    // When: two moves, one settle; then a settle with nothing moved
    key(&page, "ArrowLeft");
    key(&page, "ArrowLeft");
    settle(&page);
    settle(&page);

    // Then: once, and carrying no value, as the DOM's does
    assert_eq!(*page.log.borrow(), ["selectionchange \"\""]);
}

#[test]
fn a_removed_field_takes_its_editor_with_it() {
    // Given
    let page = page();
    click(&page, [120.0, 12.0]);

    // When
    page.doc.borrow_mut().remove(page.name);
    settle(&page);

    // Then: no selection to report, nothing fired
    assert_eq!(page.doc.borrow().active_element(), None);
    assert!(
        page.log
            .borrow()
            .iter()
            .all(|e| !e.starts_with("selectionchange"))
    );
}

#[test]
fn the_value_attribute_reaches_the_editor_until_the_field_is_edited() {
    // Given: the field's editor made by a click
    let page = page();
    click(&page, [120.0, 12.0]);

    // When: the attribute changes, a letter is typed, the attribute changes again
    page.doc
        .borrow_mut()
        .set_attribute(page.name, "value", "Cool");
    type_text(&page, "x");
    page.doc
        .borrow_mut()
        .set_attribute(page.name, "value", "Hot");

    // Then: typing went into the attribute's text; once edited, the
    // attribute no longer moves the value, as the DOM's dirty value
    assert_eq!(value(&page, page.name), "Coolx");
}

#[test]
fn a_field_that_stops_being_a_text_field_fires_no_selectionchange() {
    // Given: a selectionchange owed by a click
    let page = page();
    click(&page, [120.0, 12.0]);
    page.log.borrow_mut().clear();

    // When: it becomes a number field before the settle
    page.doc
        .borrow_mut()
        .set_attribute(page.name, "type", "number");
    settle(&page);

    // Then
    assert!(page.log.borrow().is_empty(), "{:?}", page.log.borrow());
}

#[test]
fn a_value_set_by_a_signal_puts_the_caret_at_its_end() {
    // Given: the caret at the start of "Warm"
    let page = page();
    click(&page, [4.0, 12.0]);
    assert_eq!(selection(&page, page.name), (0, 0));

    // When: a signal sets a longer text, as Chromium's `value =` does
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", PropertyValue::Text("Alabama".into()));

    // Then
    assert_eq!(selection(&page, page.name), (7, 7));
}
