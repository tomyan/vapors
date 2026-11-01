//! The handler's part in text input: the focused field mirrored by the
//! web page's textarea, or the system IME allowed and placed for it,
//! and the IME's events as the field's edits (E6).

use super::Handler;

impl Handler {
    /// The focused text field for the platform's text input: the web
    /// page's textarea mirrors it; natively the system IME is allowed
    /// while a field has focus, its candidate window placed at the
    /// caret, and its composition kept over the field.
    pub(super) fn mirror(&self, field: Option<&crate::TextField>) {
        #[cfg(target_family = "wasm")]
        if let Some(bridge) = &self.text_bridge {
            bridge.mirror(field);
        }
        #[cfg(not(target_family = "wasm"))]
        {
            if let Some(window) = &self.window {
                crate::ime_window::allow(window, field);
            }
            self.composition.borrow_mut().set_field(field.cloned());
        }
    }

    /// The system IME's event, as the edit it makes to the field.
    #[cfg(not(target_family = "wasm"))]
    pub(super) fn ime(&mut self, ime: &winit::event::Ime) {
        let edit = self.composition.borrow_mut().edit(ime);
        if let Some(edit) = edit {
            self.app.event(edit, &self.platform);
        }
    }
}
