//! What the handler's entry points get before an event: a held clear
//! applied before any event but the IME's own, and the IME allowed at
//! the window's first event for a field set before it opened.

use winit::event::{Ime, WindowEvent};

use super::*;
use crate::ime_window::tests::Told;

fn warm() -> TextField {
    TextField {
        id: 1,
        text: "Warm".into(),
        selection: (4, 4),
        area: [10.0, 20.0, 1.0, 16.0],
    }
}

/// Composing "a" after "Warm", then the IME's empty preedit (a cancel),
/// held.
fn cancelled(window: &Told) -> ImeState {
    let mut state = ImeState::default();
    state.mirror(Some(window), Some(&warm()));
    let preedit = WindowEvent::Ime(Ime::Preedit("a".into(), Some((1, 1))));
    assert!(state.window_event(window, &preedit).is_some());
    let echo = TextField {
        text: "Warma".into(),
        selection: (5, 5),
        ..warm()
    };
    state.mirror(Some(window), Some(&echo));
    let cleared = WindowEvent::Ime(Ime::Preedit(String::new(), None));
    assert!(state.window_event(window, &cleared).is_none());
    state
}

fn text_of(event: Option<Event>) -> String {
    match event {
        Some(Event::TextEdited { text, .. }) => text,
        other => panic!("not an edit: {other:?}"),
    }
}

#[test]
fn a_held_clear_is_applied_before_any_window_event_a_redraw_included() {
    for event in [WindowEvent::Focused(false), WindowEvent::RedrawRequested] {
        // Given
        let window = Told::default();
        let mut state = cancelled(&window);

        // When
        let edit = state.window_event(&window, &event);

        // Then: the preedit goes before the event
        assert_eq!(text_of(edit), "Warm", "{event:?}");
    }
}

#[test]
fn a_held_clear_is_applied_before_a_message_of_the_loop() {
    // Given
    let window = Told::default();
    let mut state = cancelled(&window);

    // When
    let edit = state.message();

    // Then
    assert_eq!(text_of(edit), "Warm");
}

#[test]
fn a_field_set_before_the_window_opened_gets_the_ime_at_its_first_event() {
    // Given: the field set with no window
    let mut state = ImeState::default();
    state.mirror(None::<&Told>, Some(&warm()));

    // When: the window's first event
    let window = Told::default();
    drop(state.window_event(&window, &WindowEvent::RedrawRequested));
    drop(state.window_event(&window, &WindowEvent::RedrawRequested));

    // Then: told once
    assert_eq!(
        *window.0.borrow(),
        ["allowed true", "area [10.0, 20.0] [1.0, 16.0]"]
    );
}
