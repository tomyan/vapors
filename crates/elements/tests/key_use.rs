//! Whether the document used a key (`Document::key_pressed`'s answer): a
//! control's key is used, so an app does not also act on it; a key no
//! control takes, such as a shortcut, is not, so the app gets it, as a
//! browser gives the page's shortcuts the keys no control takes.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::range_page::{key, page, value};
use support::{document, viewport};
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Modifiers};

fn control() -> Modifiers {
    Modifiers {
        control: true,
        ..Modifiers::default()
    }
}

#[test]
fn a_focused_ranges_arrow_is_used_and_a_shortcut_is_not() {
    // Given: the range focused by Tab
    let page = page();
    key(&page, "Tab");

    // When
    let arrow = Document::key_pressed(&page.doc, "ArrowRight", Modifiers::default());
    let undo = Document::key_pressed(&page.doc, "z", control());
    let letter = Document::key_pressed(&page.doc, "e", Modifiers::default());

    // Then
    assert!(arrow);
    assert_eq!(value(&page, page.level), "21");
    assert!(!undo, "Control+Z is the app's");
    assert!(!letter, "E is the app's while a range has focus");
}

#[test]
fn tab_is_used_and_a_key_with_nothing_focused_is_not() {
    // Given
    let page = page();

    // Then
    assert!(!Document::key_pressed(&page.doc, "z", Modifiers::default()));
    assert!(Document::key_pressed(
        &page.doc,
        "Tab",
        Modifiers::default()
    ));
}

#[test]
fn a_prevented_keydown_is_used() {
    // Given
    let page = page();
    let body = page.doc.borrow().body();
    page.doc.borrow_mut().listen(
        body,
        "keydown",
        Box::new(|e: &mut Event| e.prevent_default()),
    );

    // Then
    assert!(Document::key_pressed(&page.doc, "z", control()));
}

/// A focused text field takes typing, its moves, and its undo
/// (Control+Z, vapors #92); a Control key it has no binding for is the
/// app's.
/// A focused `<input type=…>` of `kind`, laid out.
fn focused_field(kind: &str) -> Rc<RefCell<Document>> {
    let mut doc = document();
    doc.add_stylesheet("app.css", "body { display: flex; } input { width: 6em; }")
        .expect("parses");
    let body = doc.body();
    let field = doc.create_element("input");
    doc.set_attribute(field, "type", kind);
    doc.append(body, field);
    doc.update(viewport(200.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    Document::focus(&doc, Some(field));
    doc
}

#[test]
fn a_focused_number_field_uses_the_keys_it_types_and_not_control_z() {
    // Given
    let doc = focused_field("number");

    // Then
    for key in ["1", "-", ".", "e"] {
        assert!(
            Document::key_pressed(&doc, key, Modifiers::default()),
            "{key}"
        );
    }
    assert!(!Document::key_pressed(&doc, "z", control()));
}

#[test]
fn named_keys_in_a_focused_text_field_are_the_apps() {
    // Given
    let doc = focused_field("text");

    // Then
    for key in ["F1", "F12", "Escape"] {
        assert!(
            !Document::key_pressed(&doc, key, Modifiers::default()),
            "{key}"
        );
    }
}

#[test]
fn a_focused_text_field_uses_a_typing_key_and_not_an_unbound_control_key() {
    // Given
    let doc = focused_field("text");

    // Then
    assert!(Document::key_pressed(&doc, "e", Modifiers::default()));
    assert!(Document::key_pressed(
        &doc,
        "ArrowLeft",
        Modifiers::default()
    ));
    assert!(
        Document::key_pressed(&doc, "z", control()),
        "the field's undo"
    );
    assert!(!Document::key_pressed(&doc, "k", control()));
}
