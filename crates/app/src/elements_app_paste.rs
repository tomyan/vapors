//! A paste on the desktop (E6b): `ElementsApp` reads its clipboard for
//! the document through its `DocumentInput` (`document_input_paste.rs`),
//! which owns the clipboard.

use vapors_platform::Clipboard;

use super::ElementsApp;

impl ElementsApp {
    /// Reads the clipboard through `clipboard` for a paste into a text
    /// field, in place of the system's.
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.input = self.input.with_clipboard(clipboard);
        self
    }

    /// Pastes each clipboard answer that has arrived into the field it
    /// was read for; the frame does this first, and a test without a
    /// window may.
    pub fn deliver_pastes(&mut self) {
        self.input.deliver_pastes(&self.document);
    }
}
