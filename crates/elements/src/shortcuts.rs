//! Select-all in a field: A with Command on macOS and with Control
//! elsewhere, and neither the other of the two nor Alt held, as Chromium
//! has it (on macOS Control+A moves to the line's start, which vapors
//! does not take). The app says which platform it runs on
//! (`Document::set_shortcut_modifier`; vapors-app asks
//! `vapors_platform::shortcut_is_command`); Control until it does.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::NodeId;

use crate::document::Document;
use crate::keys::Modifiers;

/// The modifier the platform's shortcuts take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShortcutModifier {
    /// Control: Linux, Windows, Chrome OS, Android.
    #[default]
    Control,
    /// Command (Meta): macOS, iPhone, iPad.
    Command,
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
    let mut document = document.borrow_mut();
    if document.number_field(node).is_some() {
        document.select_number_text(node);
    } else {
        // the method path is not general over the driver's lifetime
        #[allow(clippy::redundant_closure_for_method_calls)]
        document.drive(node, |d| d.select_all());
    }
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
        let (shortcut, other) = if self.shortcut_modifier == ShortcutModifier::Command {
            (modifiers.meta, modifiers.control)
        } else {
            (modifiers.control, modifiers.meta)
        };
        matches!(key, "a" | "A") && shortcut && !other && !modifiers.alt
    }
}
