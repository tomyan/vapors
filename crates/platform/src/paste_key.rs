//! Which key press is a paste: V with Control, or with Command on macOS,
//! and nothing else held; a first press, not a repeat. V is the logical
//! key on a Latin layout (Dvorak's V is where it says), else the
//! physical V key (on a Cyrillic layout the logical key is "м").
//!
//! The press becomes `Event::Paste` in place of its `Character`; the
//! app, where focus lives, gives it to a focused text field first
//! (`ElementsApp`, E6b).

use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, PhysicalKey};

use crate::event::{Event, Modifiers};

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
    /// The text the press produces, if any.
    pub text: Option<&'a str>,
}

/// Whether `press` is a paste on a platform whose shortcut modifier is
/// Command (`mac`) or Control: the press as the DOM names it, through
/// [`paste_shortcut`], the one rule.
pub fn is_paste(press: &Press<'_>, mac: bool) -> bool {
    let Event::Key {
        key,
        code,
        modifiers,
        ..
    } = crate::key_event::key_event(
        press.logical,
        press.physical,
        press.state,
        press.repeat,
        press.modifiers,
    )
    else {
        unreachable!("key_event makes a key")
    };
    press.state == ElementState::Pressed
        && paste_shortcut(&key, &code, modifiers, press.repeat, mac)
}

/// Whether a key going down, as the DOM names it, is the paste shortcut:
/// V with Command on macOS (`mac`), with Control elsewhere, and nothing
/// else held; a first press, not a repeat. V is `key` on a Latin layout
/// (Dvorak's V is where it says), else the key at the V place (`code`
/// `KeyV`: on a Cyrillic layout `key` is "м"). The platform makes
/// `Event::Paste` by this rule, and an app asks it whether a key-down is
/// the paste that follows.
#[must_use]
pub fn paste_shortcut(
    key: &str,
    code: &str,
    modifiers: Modifiers,
    repeat: bool,
    mac: bool,
) -> bool {
    let shortcut = if mac {
        Modifiers {
            meta: true,
            ..Modifiers::default()
        }
    } else {
        Modifiers {
            control: true,
            ..Modifiers::default()
        }
    };
    !repeat && modifiers == shortcut && is_v(key, code)
}

fn is_v(key: &str, code: &str) -> bool {
    // A single Latin letter; a control character ("\u{16}", which Ctrl+V
    // can produce), another script, or a named or dead key
    // ("Unidentified", "Dead") falls to the physical key.
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphabetic() => c.eq_ignore_ascii_case(&'v'),
        _ => code == "KeyV",
    }
}

#[cfg(test)]
#[path = "paste_key_tests.rs"]
mod tests;
