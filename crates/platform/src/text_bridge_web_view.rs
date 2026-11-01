//! The textarea as its listeners read it: its state as an edit, and its
//! keys as the app's.

use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use super::View;
use crate::event::{Event, Modifiers};

impl View {
    /// The textarea's state as an edit of the text last mirrored, which
    /// it then mirrors.
    pub(super) fn edited(&self) -> Event {
        let text = self.textarea.value();
        let start = self.textarea.selection_start().ok().flatten().unwrap_or(0);
        let end = self
            .textarea
            .selection_end()
            .ok()
            .flatten()
            .unwrap_or(start);
        let composing = self.shared.composing.borrow().as_ref().map(|data| {
            let length = u32::try_from(data.encode_utf16().count()).unwrap_or(u32::MAX);
            (end.saturating_sub(length), end)
        });
        let before = self.shared.mirrored.replace(text.clone());
        Event::TextEdited {
            before,
            text,
            selection: (start, end),
            composing,
        }
    }

    /// A key on the textarea, to the app as `Event::Key`; its default is
    /// prevented unless the textarea acts on it (`text_keys`).
    pub(super) fn key(&self, event: &web_sys::Event, pressed: bool, send: &dyn Fn(Event)) {
        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        let composing = key.is_composing() || self.shared.composing.borrow().is_some();
        let name = key.key();
        if pressed
            && !crate::text_keys::textarea_keeps(&name, key.ctrl_key(), key.meta_key(), composing)
        {
            event.prevent_default();
        }
        if composing {
            return;
        }
        send(Event::Key {
            key: name,
            pressed,
            repeat: key.repeat(),
            modifiers: Modifiers {
                shift: key.shift_key(),
                control: key.ctrl_key(),
                alt: key.alt_key(),
                meta: key.meta_key(),
            },
        });
    }
}
