//! Select-all, undo and redo in a field (undo and redo in a text field
//! only, `text_undo`). Select-all: A with Command on macOS and with Control
//! elsewhere, and neither the other of the two nor Alt nor Shift held,
//! as Chromium has it (number.events pins Control+Shift+A; on macOS
//! Control+A moves to the line's start, which vapors
//! does not take). The app says which platform it runs on
//! (`Document::set_shortcut_modifier`; vapors-app asks
//! `vapors_platform::shortcut_is_command`). Until it does, natively the
//! target OS decides; a wasm build names no OS, so a web host must set
//! it before the first key: `ElementsApp::new` does, and a host using
//! `DocumentInput` alone does it itself.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::keys::Modifiers;

/// The modifier the platform's shortcuts take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutModifier {
    /// Control: Linux, Windows, Chrome OS, Android.
    Control,
    /// Command (Meta): macOS, iPhone, iPad.
    Command,
}

impl Default for ShortcutModifier {
    /// The target OS's: Command on macOS, else Control (on the web, until
    /// the app sets the browser's).
    fn default() -> Self {
        if cfg!(target_os = "macos") {
            Self::Command
        } else {
            Self::Control
        }
    }
}

/// Selects the whole text of `node`, a focused field, when `key` with
/// `modifiers` is select-all; whether it was.
pub(crate) fn selected_all(
    document: &Rc<RefCell<Document>>,
    node: NodeId,
    key: &str,
    modifiers: Modifiers,
) -> bool {
    if !document.borrow().is_select_all(key, modifiers) {
        return false;
    }
    // the method path is not general over the driver's lifetime
    #[allow(clippy::redundant_closure_for_method_calls)]
    document
        .borrow_mut()
        .move_selection(node, |d| d.select_all());
    true
}

impl Document {
    /// Shortcuts take `modifier`.
    pub fn set_shortcut_modifier(&mut self, modifier: ShortcutModifier) {
        self.shortcut_modifier = modifier;
    }

    /// The modifier shortcuts take.
    #[must_use]
    pub fn shortcut_modifier(&self) -> ShortcutModifier {
        self.shortcut_modifier
    }

    /// Whether `key` with `modifiers` is select-all on this platform:
    /// the one answer for a text field and a number field.
    fn is_select_all(&self, key: &str, modifiers: Modifiers) -> bool {
        matches!(key, "a" | "A") && self.shortcut_held(modifiers) && !modifiers.shift
    }

    /// Whether `key` with `modifiers` is a text field's undo (`Some(false)`)
    /// or redo (`Some(true)`) on this platform: Z with the shortcut
    /// modifier, with Shift to redo; Control+Y redoes too, but not
    /// Command+Y (on macOS Chromium leaves it to the browser's history).
    pub(crate) fn history_shortcut(&self, key: &str, modifiers: Modifiers) -> Option<bool> {
        if !self.shortcut_held(modifiers) {
            return None;
        }
        let control_y = self.shortcut_modifier == ShortcutModifier::Control;
        match key {
            "z" | "Z" => Some(modifiers.shift),
            "y" | "Y" if control_y && !modifiers.shift => Some(true),
            _ => None,
        }
    }

    /// Whether the platform's shortcut modifier is held, and neither the
    /// other platform's nor Alt.
    fn shortcut_held(&self, modifiers: Modifiers) -> bool {
        let (shortcut, other) = if self.shortcut_modifier == ShortcutModifier::Command {
            (modifiers.meta, modifiers.control)
        } else {
            (modifiers.control, modifiers.meta)
        };
        shortcut && !other && !modifiers.alt
    }
}
