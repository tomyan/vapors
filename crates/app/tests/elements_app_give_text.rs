//! The platform's text input through `ElementsApp::give`: an edit or a
//! paste the focused field took is the document's (`Route::Document`),
//! and one the document refused comes back as a `GiveError` saying
//! what happened, for the wrapping app to stop on.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, GiveError, Route};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Platform};
use vapors_text::FontSystem;

/// A text field holding "Warm", focused when `focused`.
fn field(focused: bool) -> (ElementsApp, Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "input { display: block; width: 160px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let (body, input) = (doc.body(), doc.create_element("input"));
    doc.set_attribute(input, "value", "Warm");
    doc.append(body, input);
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    if focused {
        Document::focus(&doc, Some(input));
    }
    (ElementsApp::new(Rc::clone(&doc)), doc, input)
}

fn edited(before: &str, text: &str) -> Event {
    let end = u32::try_from(text.len()).expect("short");
    Event::TextEdited {
        before: before.to_owned(),
        text: text.to_owned(),
        selection: (end, end),
        composing: None,
    }
}

fn value(doc: &Rc<RefCell<Document>>, input: NodeId) -> String {
    doc.borrow().value(input).expect("an input has a value")
}

#[test]
fn an_edit_the_focused_field_took_is_the_documents() {
    // Given
    let (mut app, doc, input) = field(true);

    // When
    let given = app.give(&edited("Warm", "Warmer"), &Platform::detached());

    // Then
    assert_eq!(given, Ok(Route::Document));
    assert_eq!(value(&doc, input), "Warmer");
}

#[test]
fn a_paste_the_focused_field_took_is_the_documents() {
    // Given
    let (mut app, doc, input) = field(true);

    // When
    let pasted = Event::PastedText(" tone".into());
    let given = app.give(&pasted, &Platform::detached());

    // Then
    assert_eq!(given, Ok(Route::Document));
    assert!(
        value(&doc, input).contains("tone"),
        "{}",
        value(&doc, input)
    );
}

#[test]
fn an_edit_against_a_text_the_field_no_longer_holds_is_refused() {
    // Given
    let (mut app, doc, input) = field(true);

    // When
    let given = app.give(&edited("Cool", "Cooler"), &Platform::detached());

    // Then
    assert!(matches!(given, Err(GiveError::EditRefused(_))), "{given:?}");
    assert_eq!(value(&doc, input), "Warm");
}

#[test]
fn a_paste_with_no_text_field_focused_is_refused() {
    // Given
    let (mut app, doc, input) = field(false);

    // When
    let pasted = Event::PastedText("tone".into());
    let given = app.give(&pasted, &Platform::detached());

    // Then
    assert_eq!(given, Err(GiveError::PasteWithoutField));
    assert_eq!(value(&doc, input), "Warm");
}
