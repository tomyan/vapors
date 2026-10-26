//! Which key press is a paste: V with Control, or with Command on macOS,
//! and nothing else held; a first press, not a repeat. V is the logical
//! key on a Latin layout (Dvorak's V is where it says), else the
//! physical V key (on a Cyrillic layout the logical key is "м").
//!
//! The press becomes `Event::Paste` in place of its `Character`: no
//! element takes typed text yet (text inputs are F4); when one does, a
//! focused input takes Ctrl+V first, decided where focus lives.

use winit::event::ElementState;
use winit::keyboard::{Key, KeyCode, ModifiersState, PhysicalKey};

/// One key event's facts that decide a paste.
pub struct Press<'a> {
    /// Pressed or released.
    pub state: ElementState,
    /// An auto-repeat of a held key.
    pub repeat: bool,
    /// The key as the layout names it.
    pub logical: &'a Key,
    /// The key's place on the keyboard.
    pub physical: PhysicalKey,
    /// The modifier keys held.
    pub modifiers: ModifiersState,
}

/// Whether `press` is a paste on a platform whose shortcut modifier is
/// Command (`mac`) or Control.
pub fn is_paste(press: &Press<'_>, mac: bool) -> bool {
    let shortcut = if mac {
        ModifiersState::SUPER
    } else {
        ModifiersState::CONTROL
    };
    press.state == ElementState::Pressed
        && !press.repeat
        && press.modifiers == shortcut
        && is_v(press)
}

fn is_v(press: &Press<'_>) -> bool {
    match press.logical {
        // A Latin letter; a control character ("\u{16}", which Ctrl+V can
        // produce) or another script falls to the physical key.
        Key::Character(text) if text.chars().all(|c| c.is_ascii_alphabetic()) => {
            text.eq_ignore_ascii_case("v")
        }
        _ => press.physical == PhysicalKey::Code(KeyCode::KeyV),
    }
}

#[cfg(test)]
#[path = "paste_key_tests.rs"]
mod tests;
