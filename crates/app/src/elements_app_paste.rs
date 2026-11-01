//! A paste on the desktop (E6b): the platform sends `Event::Paste` for
//! Control+V (Command+V on macOS) whatever has focus, and the app hands
//! it to the focused text field first. The clipboard is read off the
//! event loop (`Clipboard::read_text`): its answer, or why there is none
//! once the clipboard's deadline passes (`CLIPBOARD_DEADLINE`, 3 s),
//! comes back on a channel and wakes the loop, and the next frame
//! pastes it into the field it was read for. With no text field focused
//! the clipboard is not read: an elements app has no paste of its own
//! yet.

use std::sync::mpsc;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::{Clipboard, ClipboardText, Platform, report_error};

use super::ElementsApp;

/// The clipboard reads a paste started, and where their answers come.
pub(super) struct Pastes {
    sender: mpsc::Sender<ClipboardText>,
    answers: mpsc::Receiver<ClipboardText>,
    /// The field each read in flight is for, oldest first.
    fields: std::collections::VecDeque<NodeId>,
}

impl Default for Pastes {
    fn default() -> Self {
        let (sender, answers) = mpsc::channel();
        Self {
            sender,
            answers,
            fields: std::collections::VecDeque::new(),
        }
    }
}

impl ElementsApp {
    /// Reads the clipboard through `clipboard` for a paste into a text
    /// field, in place of the system's.
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.clipboard = clipboard;
        self
    }

    /// A paste: the clipboard's text is read for the focused text field.
    pub(super) fn paste(&mut self, platform: &Platform) {
        let focused = self.document.borrow().active_element();
        let Some(node) = focused.filter(|_| self.document.borrow().text_field_focused()) else {
            return;
        };
        self.pastes.fields.push_back(node);
        let (sender, waker) = (self.pastes.sender.clone(), platform.waker());
        self.clipboard.read_text(Box::new(move |text| {
            // a closed loop has no app left to paste into
            if sender.send(text).is_ok() {
                let _ = waker.wake();
            }
        }));
    }

    /// Pastes each clipboard answer that has arrived into the field it
    /// was read for; the frame does this first, and a test without a
    /// window may.
    pub fn deliver_pastes(&mut self) {
        while let Ok(text) = self.pastes.answers.try_recv() {
            let field = self.pastes.fields.pop_front();
            self.pasted(field, text);
        }
    }

    fn pasted(&self, field: Option<NodeId>, text: ClipboardText) {
        match text {
            ClipboardText::Text(text) if field == self.document.borrow().active_element() => {
                Document::text_pasted(&self.document, &text);
            }
            ClipboardText::Text(_) => report_error(
                "the clipboard's text arrived after the text field it was read for lost focus, so it was not pasted",
            ),
            ClipboardText::NoText => {}
            ClipboardText::Unreadable(why) => report_error(&format!(
                "the clipboard could not be read for a paste: {why}"
            )),
        }
    }
}
