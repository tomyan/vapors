//! A paste into the document (E6b): the platform sends `Event::Paste`
//! for the paste shortcut (`vapors_platform::paste_shortcut`: Command+V
//! on macOS, Control+V elsewhere) whatever has focus. While one of the
//! document's text fields has focus, the paste is the document's:
//! [`DocumentInput::give`] reads the clipboard off the event loop
//! (`Clipboard::read_text`), its answer, or why there is none once the
//! clipboard's deadline passes, comes back on a channel and wakes the
//! loop, and [`DocumentInput::deliver_pastes`] (each frame, before the
//! update, and at the start of each `give`) pastes it into the field it
//! was read for. With no text field focused the paste is the host's (an
//! image paste, say), and the clipboard is not read.
//!
//! RELAXED on the web: the browser has its own paste event and the
//! platform sends no `Event::Paste`, so a card's text field is not
//! pasted into on the web, and a web host's own paste (touché's
//! `drop.js`) must not paste an image while a card field has focus. A
//! web host asks first: `give(&Event::Paste, ..)` answers
//! [`Route::Document`](super::Route::Document) while a field has focus
//! (it reads no clipboard on the web), as does
//! `Document::text_field_focused()`. It retires when touché wires its
//! textarea paste path (#94) to card fields, through
//! `Document::text_pasted`.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use vapors_dom::NodeId;
use vapors_elements::Document;
use vapors_platform::{Clipboard, ClipboardText, Event, Platform, report_error};

use super::DocumentInput;

/// The clipboard, and where its answers to pastes come, each with the
/// field it was read for: a read refused at once may overtake a slow one.
pub(super) struct Pastes {
    clipboard: Box<dyn Clipboard>,
    sender: mpsc::Sender<(NodeId, ClipboardText)>,
    answers: mpsc::Receiver<(NodeId, ClipboardText)>,
}

impl Default for Pastes {
    fn default() -> Self {
        let (sender, answers) = mpsc::channel();
        Self {
            clipboard: Box::new(vapors_platform::SystemClipboard::default()),
            sender,
            answers,
        }
    }
}

impl DocumentInput {
    /// Reads the clipboard through `clipboard` for a paste into a text
    /// field, in place of the system's.
    #[must_use]
    pub fn with_clipboard(mut self, clipboard: Box<dyn Clipboard>) -> Self {
        self.pastes.clipboard = clipboard;
        self
    }

    /// A paste, when one of `document`'s text fields has focus: its text
    /// is read for that field. Else nothing, as `route` gave it the host.
    pub(super) fn paste_if(
        &mut self,
        document: &Rc<RefCell<Document>>,
        event: &Event,
        platform: &Platform,
    ) {
        if !matches!(event, Event::Paste) {
            return;
        }
        let focused = document.borrow().active_element();
        let Some(node) = focused.filter(|_| document.borrow().text_field_focused()) else {
            return;
        };
        let (sender, waker) = (self.pastes.sender.clone(), platform.waker());
        self.pastes.clipboard.read_text(Box::new(move |text| {
            // a closed loop has no app left to paste into
            if sender.send((node, text)).is_ok() {
                let _ = waker.wake();
            }
        }));
    }

    /// Pastes each clipboard answer that has arrived into the field it
    /// was read for: call it each frame, before the update.
    pub fn deliver_pastes(&mut self, document: &Rc<RefCell<Document>>) {
        while let Ok((field, text)) = self.pastes.answers.try_recv() {
            pasted(document, field, text);
        }
    }
}

fn pasted(document: &Rc<RefCell<Document>>, field: NodeId, text: ClipboardText) {
    match text {
        ClipboardText::Text(text) if Some(field) == document.borrow().active_element() => {
            Document::text_pasted(document, &text);
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

#[cfg(test)]
#[path = "document_input_paste_tests.rs"]
mod tests;
