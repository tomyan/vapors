//! A text field's own undo (E6b), grouped as Chromium groups it: a run
//! of typing is one step, a run of Backspace another, typing after a
//! deletion joins it, a moved caret ends the run; undo restores the
//! text and the selection before the step, redo the text and selection
//! after it, each with `input`. The order against Chromium is pinned by
//! the corpus's `undo.events`.

mod support;

use support::text_page::{
    click, control, key, key_with, page, selection, settle, type_text, value,
};
use vapors_dom::{Dom, PropertyValue};
use vapors_elements::Modifiers;

fn undo(page: &support::text_page::Page) {
    key_with(page, "z", control());
}

fn redo(page: &support::text_page::Page) {
    let shift_control = Modifiers {
        shift: true,
        ..control()
    };
    key_with(page, "Z", shift_control);
}

/// The first field focused with the caret after "Warm".
fn at_end() -> support::text_page::Page {
    let page = page();
    click(&page, [120.0, 12.0]);
    page
}

#[test]
fn a_run_of_typing_is_one_step_and_a_moved_caret_ends_it() {
    // Given: "th", the caret moved, "x"
    let page = at_end();
    type_text(&page, "th");
    key(&page, "ArrowLeft");
    type_text(&page, "x");

    // When
    undo(&page);
    let first = (value(&page, page.name), selection(&page, page.name));
    undo(&page);

    // Then: "x" went alone, the caret where it was typed; then "th"
    assert_eq!(first, ("Warmth".to_owned(), (5, 5)));
    assert_eq!(value(&page, page.name), "Warm");
    assert_eq!(selection(&page, page.name), (4, 4));
}

#[test]
fn undo_with_nothing_to_undo_changes_nothing_and_fires_nothing() {
    // Given
    let page = at_end();
    page.log.borrow_mut().clear();

    // When
    undo(&page);

    // Then: the field's first text is not a step
    assert_eq!(value(&page, page.name), "Warm");
    assert!(page.log.borrow().iter().all(|e| !e.starts_with("input")));
}

#[test]
fn redo_restores_the_text_and_selection_after_the_step() {
    // Given: "th" typed and undone
    let page = at_end();
    type_text(&page, "th");
    undo(&page);
    page.log.borrow_mut().clear();

    // When
    redo(&page);

    // Then
    assert_eq!(value(&page, page.name), "Warmth");
    assert_eq!(selection(&page, page.name), (6, 6));
    assert_eq!(page.log.borrow()[0], "input \"Warmth\"");
}

#[test]
fn control_y_redoes_too() {
    // Given
    let page = at_end();
    type_text(&page, "th");
    undo(&page);

    // When
    key_with(&page, "y", control());

    // Then
    assert_eq!(value(&page, page.name), "Warmth");
}

#[test]
fn backspaces_are_one_step_and_typing_after_them_joins_it() {
    // Given: "ab" typed, then a Backspace and "c": two steps
    let page = at_end();
    type_text(&page, "ab");
    key(&page, "Backspace");
    type_text(&page, "c");

    // When
    undo(&page);

    // Then: the Backspace and "c" went together
    assert_eq!(value(&page, page.name), "Warmab");
    assert_eq!(selection(&page, page.name), (6, 6));
}

#[test]
fn undoing_typing_over_a_selection_selects_the_text_again() {
    // Given: the whole text selected and typed over
    let page = at_end();
    key_with(&page, "a", control());
    type_text(&page, "Q");

    // When
    undo(&page);

    // Then
    assert_eq!(value(&page, page.name), "Warm");
    assert_eq!(selection(&page, page.name), (0, 4));
}

#[test]
fn a_new_edit_after_undo_drops_what_could_be_redone() {
    // Given: "th" typed and undone, then "x"
    let page = at_end();
    type_text(&page, "th");
    undo(&page);
    type_text(&page, "x");

    // When
    redo(&page);

    // Then
    assert_eq!(value(&page, page.name), "Warmx");
}

#[test]
fn a_signal_setting_the_text_forgets_the_undo() {
    // Given: "th" typed, then a signal sets the text
    let page = at_end();
    type_text(&page, "th");
    page.doc
        .borrow_mut()
        .set_property(page.name, "value", PropertyValue::Text("Cool".into()));

    // When
    undo(&page);

    // Then: the signal's text stays
    assert_eq!(value(&page, page.name), "Cool");
}

#[test]
fn undo_of_text_before_the_caret_fires_selectionchange_even_where_it_was() {
    // Given: "ab", a Backspace and "c": the caret at 6 before and after,
    // the "c" undone at 5, before it
    let page = at_end();
    type_text(&page, "ab");
    key(&page, "Backspace");
    type_text(&page, "c");
    settle(&page);
    page.log.borrow_mut().clear();

    // When
    undo(&page);
    settle(&page);

    // Then
    assert_eq!(selection(&page, page.name), (6, 6));
    assert!(
        page.log
            .borrow()
            .contains(&"selectionchange \"\"".to_owned())
    );
}

#[test]
fn undo_of_text_at_the_caret_that_leaves_it_in_place_fires_no_selectionchange() {
    // Given: the caret at 2, and a Delete there
    let page = at_end();
    key(&page, "Home");
    key(&page, "ArrowRight");
    key(&page, "ArrowRight");
    key(&page, "Delete");
    settle(&page);
    page.log.borrow_mut().clear();

    // When: the "r" goes back at the caret, not before it
    undo(&page);
    settle(&page);

    // Then: as Chromium 155 (undo.events)
    assert_eq!(value(&page, page.name), "Warm");
    assert_eq!(selection(&page, page.name), (2, 2));
    assert!(
        !page
            .log
            .borrow()
            .iter()
            .any(|line| line.starts_with("selectionchange"))
    );
}
