//! A winit key event as [`Event::Key`] (3-E2a): the key named as the DOM
//! names it, with the modifier keys held.

use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::event::{Event, Modifiers};

/// `logical` going down or up, with `held` modifiers.
pub(crate) fn key_event(
    logical: &Key,
    state: ElementState,
    repeat: bool,
    held: ModifiersState,
) -> Event {
    Event::Key {
        key: dom_name(logical),
        pressed: state == ElementState::Pressed,
        repeat,
        modifiers: Modifiers {
            shift: held.shift_key(),
            control: held.control_key(),
            alt: held.alt_key(),
            meta: held.super_key(),
        },
    }
}

/// The DOM's `KeyboardEvent.key`: winit's named keys are spelled as the
/// DOM spells them (`ArrowLeft`, `Escape`), except Space, which the DOM
/// names by its character.
fn dom_name(logical: &Key) -> String {
    match logical {
        Key::Character(text) => text.to_string(),
        Key::Named(NamedKey::Space) => " ".to_owned(),
        Key::Named(named) => format!("{named:?}"),
        Key::Unidentified(_) | Key::Dead(_) => "Unidentified".to_owned(),
    }
}
