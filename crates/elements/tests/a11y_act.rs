//! Acting on a node found by role and name (R1), through a person's
//! input path: a press is the pointer at the node's centre, typing and
//! keys go to the node focused first. Disabled, hidden and covered
//! nodes are refused by name.

mod support;

use support::drive::{log, node, shared, target, update};
use support::roles_page::{button, labelled, page};
use support::viewport;
use vapors_elements::{AxError, Document, Modifiers};

#[test]
fn a_press_moves_the_pointer_to_the_centre_and_clicks() {
    // Given
    let doc = shared(page());
    let apply = node(&doc, "button", "Apply");
    let events = log(&doc, apply, &["pointerdown", "pointerup", "click"]);

    // When
    Document::ax_press(&doc, &target("button", "Apply")).expect("pressed");

    // Then: the events a person's click fires, and the press focused it
    assert_eq!(*events.borrow(), ["pointerdown", "pointerup", "click"]);
    assert_eq!(doc.borrow().active_element(), Some(apply));
}

#[test]
fn a_press_checks_a_checkbox() {
    // Given
    let doc = shared(page());

    // When
    Document::ax_press(&doc, &target("checkbox", "Mono")).expect("pressed");
    update(&doc);

    // Then
    let mono = doc
        .borrow()
        .ax_find("checkbox", "Mono", None)
        .expect("found");
    assert_eq!(mono.states.checked, Some(true));
}

#[test]
fn acting_on_a_disabled_node_is_refused_by_name() {
    // Given
    let doc = shared(page());

    // When
    let pressed = Document::ax_press(&doc, &target("button", "Reset"));
    let focused = Document::ax_focus(&doc, &target("checkbox", "Lock"));

    // Then
    let disabled = |role: &str, name: &str| AxError::Disabled {
        role: role.to_owned(),
        name: name.to_owned(),
    };
    assert_eq!(pressed, Err(disabled("button", "Reset")));
    assert_eq!(focused, Err(disabled("checkbox", "Lock")));
}

#[test]
fn acting_on_a_node_that_is_not_drawn_is_refused_as_hidden() {
    // Given: a button Ghost under aria-hidden, in the dialog's last row
    let mut page = page();
    let dialog = page.children(page.body())[0];
    let row = *page.children(dialog).last().expect("a row");
    button(&mut page, row, "Ghost", &[("aria-hidden", "true")]);
    page.update(viewport(640.0, 400.0)).expect("updates");
    let doc = shared(page);

    // When
    let pressed = Document::ax_press(&doc, &target("button", "Ghost"));

    // Then
    let hidden = AxError::Hidden {
        role: "button".to_owned(),
        name: "Ghost".to_owned(),
    };
    assert_eq!(pressed, Err(hidden));
}

#[test]
fn a_press_whose_centre_another_element_covers_is_refused_naming_it() {
    // Given: a div over the whole page
    let mut page = page();
    let body = page.body();
    let cover = "position: absolute; left: 0; top: 0; width: 640px; height: 400px";
    labelled(&mut page, body, "div", &[("style", cover)]);
    page.update(viewport(640.0, 400.0)).expect("updates");
    let doc = shared(page);
    let apply = node(&doc, "button", "Apply");
    let events = log(&doc, apply, &["click"]);

    // When
    let pressed = Document::ax_press(&doc, &target("button", "Apply"));

    // Then: refused, and nothing clicked
    let Err(AxError::Covered { by, .. }) = pressed else {
        panic!("covered expected, got {pressed:?}");
    };
    assert_eq!(by.as_deref(), Some("div"));
    assert!(events.borrow().is_empty());
}

#[test]
fn focus_moves_to_the_node() {
    // Given
    let doc = shared(page());

    // When
    Document::ax_focus(&doc, &target("textbox", "Note")).expect("focused");
    update(&doc);

    // Then
    let note = doc
        .borrow()
        .ax_find("textbox", "Note", None)
        .expect("found");
    assert!(note.states.focused);
}

#[test]
fn focus_on_a_node_that_takes_none_is_refused() {
    // Given
    let doc = shared(page());

    // When
    let focused = Document::ax_focus(&doc, &target("group", "Tone"));

    // Then
    let refused = AxError::NotFocusable {
        role: "group".to_owned(),
        name: "Tone".to_owned(),
    };
    assert_eq!(focused, Err(refused));
}

#[test]
fn typing_focuses_the_node_and_sends_each_key_and_its_text() {
    // Given
    let doc = shared(page());
    let note = node(&doc, "textbox", "Note");
    let events = log(&doc, note, &["keydown", "input", "keyup"]);

    // When
    Document::ax_type(&doc, &target("textbox", "Note"), "yo").expect("typed");

    // Then: focus() left the caret at the field's start
    let one_key = ["keydown", "input", "keyup"];
    assert_eq!(*events.borrow(), [one_key, one_key].concat());
    assert_eq!(doc.borrow().value(note).as_deref(), Some("yohi"));
}

#[test]
fn a_key_goes_to_the_node_focused_first() {
    // Given
    let doc = shared(page());
    let amount = node(&doc, "slider", "Amount");

    // When
    let modifiers = Modifiers::default();
    Document::ax_key(&doc, &target("slider", "Amount"), "ArrowRight", modifiers).expect("key");

    // Then: the range stepped
    assert_eq!(doc.borrow().value(amount).as_deref(), Some("51"));
}

#[test]
fn an_action_after_a_change_is_refused_until_an_update() {
    // Given: a press changed the document
    let doc = shared(page());
    Document::ax_press(&doc, &target("checkbox", "Mono")).expect("pressed");

    // When
    let again = Document::ax_press(&doc, &target("checkbox", "Mono"));

    // Then
    assert_eq!(again, Err(AxError::NotUpdated));
}
