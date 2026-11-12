//! The racer's events through `ElementsApp::give`: Tab with nothing
//! focused reaches the HUD's first control, and text input the document
//! refuses stops the racer.
use super::Racer;
use vapors_elements::Viewport;
use vapors_platform::{App, Event, Modifiers, Platform};

fn tab(pressed: bool) -> Event {
    Event::Key {
        key: "Tab".into(),
        code: "Tab".into(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

#[test]
fn tab_with_nothing_focused_reaches_the_huds_first_control() {
    // Given: laid out, as Tab visits only controls with a box
    let mut racer = Racer::new();
    let viewport = Viewport {
        width: 1100.0,
        height: 720.0,
        scale: 1.0,
    };
    racer
        .document
        .borrow_mut()
        .update(viewport)
        .expect("updates");
    let platform = Platform::detached();

    // When
    racer.event(tab(true), &platform);
    racer.event(tab(false), &platform);

    // Then: the camera toggle is the first control in the HUD
    let doc = racer.document.borrow();
    let focused = doc.active_element().expect("Tab focused a control");
    assert_eq!(doc.attribute(focused, "data-action"), Some("camera"));
    assert_eq!(racer.faults.error, None);
}

#[test]
fn an_edit_the_document_refuses_stops_the_racer() {
    // Given: no text field is focused
    let mut racer = Racer::new();

    // When
    let edit = Event::TextEdited {
        before: String::new(),
        text: "x".into(),
        selection: (1, 1),
        composing: None,
    };
    racer.event(edit, &Platform::detached());

    // Then
    let error = racer.faults.error.as_deref().unwrap_or_default();
    assert!(error.contains("text input was refused"), "{error:?}");
    assert!(racer.run.paused);
}

#[test]
fn a_paste_with_no_text_field_focused_stops_the_racer() {
    // Given
    let mut racer = Racer::new();

    // When
    racer.event(Event::PastedText("x".into()), &Platform::detached());

    // Then
    let error = racer.faults.error.as_deref().unwrap_or_default();
    assert!(error.contains("no text field focused"), "{error:?}");
}
