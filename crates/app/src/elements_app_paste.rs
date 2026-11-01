//! A paste on the desktop (E6b): the platform sends `Event::Paste` for
//! Control+V (Command+V on macOS) whatever has focus, and the app hands
//! it to the focused text field first, which reads the clipboard's text.
//! With no text field focused the clipboard is not read: an elements
//! app has no paste of its own yet. The read blocks the event loop for
//! up to the clipboard's deadline (`CLIPBOARD_DEADLINE`, 3 s) when the
//! clipboard's owner stalls.

use vapors_elements::Document;
use vapors_platform::{Clipboard, ClipboardText, report_error};

use super::ElementsApp;

impl ElementsApp {
    /// Reads the clipboard through `clipboard` for a paste into a text
    /// field, in place of the system's.
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// A paste: the focused text field takes the clipboard's text.
    pub(super) fn paste(&mut self) {
        if !self.document.borrow().text_field_focused() {
            return;
        }
        match self.clipboard.text() {
            ClipboardText::Text(text) => {
                Document::text_pasted(&self.document, &text);
            }
            ClipboardText::NoText => {}
            ClipboardText::Unreadable(why) => {
                report_error(&format!(
                    "the clipboard could not be read for a paste: {why}"
                ));
            }
        }
    }
}
