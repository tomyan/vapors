//! A paste on the desktop (E6b): `ElementsApp` reads its clipboard for
//! the document through its `DocumentInput` (`document_input_paste.rs`).

use vapors_platform::{Clipboard, Platform};

use super::ElementsApp;

impl ElementsApp {
    /// Reads the clipboard through `clipboard` for a paste into a text
    /// field, in place of the system's.
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// A paste: the clipboard's text is read for the focused text field.
    /// With none focused an elements app has no paste of its own yet.
    pub(super) fn paste(&mut self, platform: &Platform) {
        let _ = self
            .input
            .paste(&self.document, &mut *self.clipboard, platform);
    }

    /// Pastes each clipboard answer that has arrived into the field it
    /// was read for; the frame does this first, and a test without a
    /// window may.
    pub fn deliver_pastes(&mut self) {
        self.input.deliver_pastes(&self.document);
    }
}
