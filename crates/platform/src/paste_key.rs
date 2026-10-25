//! Which key press is a paste: V with Control, or with Command on macOS.
//! Shift or Alt with it is another shortcut, not a paste.

use winit::keyboard::{Key, ModifiersState};

/// Whether `key` pressed with `modifiers` is a paste on a platform whose
/// shortcut modifier is Command (`mac`) or Control.
pub fn is_paste(key: &Key, modifiers: ModifiersState, mac: bool) -> bool {
    let shortcut = if mac {
        ModifiersState::SUPER
    } else {
        ModifiersState::CONTROL
    };
    let is_v = matches!(key, Key::Character(text) if text.eq_ignore_ascii_case("v"));
    is_v && modifiers == shortcut
}

#[cfg(test)]
#[path = "paste_key_tests.rs"]
mod tests;
