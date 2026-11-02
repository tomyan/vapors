//! The keyboard facts `Input` takes, and a key press as its events:
//! the key, then a paste in place of its text, or its text.

use winit::keyboard::ModifiersState;

use crate::event::Event;
use crate::key_event::key_event;
use crate::paste_key::Press;

/// A keyboard fact [`Input::key`] takes: what `keys` reads from winit,
/// so the wiring is testable (winit's `KeyEvent` cannot be built).
pub(crate) enum Key<'a> {
    Modifiers(ModifiersState),
    Focus(bool),
    Press(Press<'a>),
}

/// A key going down or up: its [`Event::Key`], then a paste in place
/// of its text (desktop only: the web page handles paste), or its
/// text as [`Event::Character`] on a press.
pub(crate) fn pressed(press: &Press<'_>) -> Vec<Event> {
    let mut events = vec![key_event(
        press.logical,
        press.physical,
        press.state,
        press.repeat,
        press.modifiers,
    )];
    let paste = !cfg!(target_family = "wasm")
        && crate::paste_key::is_paste(press, cfg!(target_os = "macos"));
    if paste {
        events.push(Event::Paste);
    } else if press.state == winit::event::ElementState::Pressed {
        events.extend(press.text.map(|text| Event::Character(text.to_owned())));
    }
    events
}
