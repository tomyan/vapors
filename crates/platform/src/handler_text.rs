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
        self.ime.borrow_mut().mirror(self.window.as_deref(), field);
    }

    /// Before a window event: the IME's own edit, or a held clear (see
    /// `ime_state`).
    #[cfg(not(target_family = "wasm"))]
    pub(super) fn ime_window_event(&mut self, event: &winit::event::WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let edit = self.ime.borrow_mut().window_event(window.as_ref(), event);
        self.send_edit(edit);
    }

    /// Before one of the loop's own messages: a held clear.
    #[cfg(not(target_family = "wasm"))]
    pub(super) fn ime_message(&mut self) {
        let edit = self.ime.borrow_mut().message();
        self.send_edit(edit);
    }

    #[cfg(not(target_family = "wasm"))]
    fn send_edit(&mut self, edit: Option<crate::Event>) {
        if let Some(edit) = edit {
            self.app.event(edit, &self.platform);
        }
    }
}
