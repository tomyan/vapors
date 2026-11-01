//! The page's text input (E6, the web): a hidden `<textarea>` mirrors the
//! focused text field, so a phone's keyboard opens for it and an IME
//! (Android's keyboards compose every word) edits it. Its edits come
//! back as `Event::TextEdited`, its pastes as `Event::PastedText`, and
//! its keys as `Event::Key` (`text_keys` says which it acts on itself).
//! It is focused, placed at the caret and given the field's text and
//! selection when the app sets the field; set to none, it blurs. The
//! text it last took from the app is kept in its `data-vapors-text`
//! and `data-vapors-selection` attributes, which the web input oracle
//! reads as the app's field.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{CompositionEvent, HtmlTextAreaElement};
use winit::event_loop::EventLoopProxy;

use crate::event::Event;
use crate::window::Message;
use crate::{PlatformError, TextField};

/// What the listeners share: the text last mirrored and the IME's
/// composing text while it composes.
#[derive(Default)]
struct Shared {
    mirrored: RefCell<String>,
    composing: RefCell<Option<String>>,
    focused: Cell<bool>,
}

/// The textarea and its listeners, for the page's life.
pub(crate) struct TextBridge {
    textarea: HtmlTextAreaElement,
    shared: Rc<Shared>,
}

impl TextBridge {
    /// Adds the textarea to the page, hidden, with its listeners.
    ///
    /// # Errors
    /// The page has no document or body, or the element cannot be made.
    pub(crate) fn new(proxy: &EventLoopProxy<Message>) -> Result<Self, PlatformError> {
        let fail = |what: &str| PlatformError::Windowing(format!("the text input: {what}"));
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| fail("no document"))?;
        let textarea: HtmlTextAreaElement = document
            .create_element("textarea")
            .map_err(|e| fail(&format!("{e:?}")))?
            .dyn_into()
            .map_err(|_| fail("not a textarea"))?;
        drop(textarea.set_attribute("style", HIDDEN));
        drop(textarea.set_attribute("aria-hidden", "true"));
        drop(textarea.set_attribute("autocomplete", "off"));
        document
            .body()
            .ok_or_else(|| fail("no body"))?
            .append_child(&textarea)
            .map_err(|e| fail(&format!("{e:?}")))?;
        let bridge = Self {
            textarea,
            shared: Rc::default(),
        };
        bridge.listen(proxy);
        Ok(bridge)
    }

    /// Mirrors `field`, or blurs the textarea when none has focus.
    pub(crate) fn mirror(&self, field: Option<&TextField>) {
        let Some(field) = field else {
            self.shared.focused.set(false);
            drop(self.textarea.set_attribute("style", HIDDEN));
            drop(self.textarea.blur());
            return;
        };
        let [x, y, width, height] = field.area;
        let style = format!(
            "{SHOWN} left: {x}px; top: {y}px; width: {}px; height: {height}px;",
            width.max(1.0)
        );
        drop(self.textarea.set_attribute("style", &style));
        if self.textarea.value() != field.text {
            self.textarea.set_value(&field.text);
        }
        let (start, end) = field.selection;
        drop(self.textarea.set_selection_range(start, end));
        self.shared.mirrored.borrow_mut().clone_from(&field.text);
        drop(self.textarea.set_attribute("data-vapors-text", &field.text));
        drop(
            self.textarea
                .set_attribute("data-vapors-selection", &format!("{start} {end}")),
        );
        if !self.shared.focused.replace(true) {
            drop(self.textarea.focus());
        }
    }

    fn listen(&self, proxy: &EventLoopProxy<Message>) {
        self.on("input", proxy, |bridge, _event, send| send(bridge.edited()));
        self.on("compositionstart", proxy, |bridge, _event, _send| {
            *bridge.shared.composing.borrow_mut() = Some(String::new());
        });
        self.on("compositionupdate", proxy, |bridge, event, _send| {
            let data = event
                .dyn_ref::<CompositionEvent>()
                .and_then(CompositionEvent::data);
            *bridge.shared.composing.borrow_mut() = Some(data.unwrap_or_default());
        });
        self.on("compositionend", proxy, |bridge, _event, send| {
            *bridge.shared.composing.borrow_mut() = None;
            send(bridge.edited());
        });
        self.on("keydown", proxy, |bridge, event, send| {
            bridge.key(event, true, send);
        });
        self.on("keyup", proxy, |bridge, event, send| {
            bridge.key(event, false, send);
        });
        self.on("paste", proxy, |_bridge, event, send| {
            event.prevent_default();
            let text = event
                .dyn_ref::<web_sys::ClipboardEvent>()
                .and_then(web_sys::ClipboardEvent::clipboard_data)
                .and_then(|data| data.get_data("text/plain").ok());
            if let Some(text) = text {
                send(Event::PastedText(text));
            }
        });
    }

    /// Adds a listener for `name` that may send events into the loop.
    fn on(
        &self,
        name: &str,
        proxy: &EventLoopProxy<Message>,
        handle: impl Fn(&View, &web_sys::Event, &dyn Fn(Event)) + 'static,
    ) {
        let view = View {
            textarea: self.textarea.clone(),
            shared: Rc::clone(&self.shared),
        };
        let proxy = proxy.clone();
        let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            let send = |event: Event| crate::tasks::send(&proxy, Message::Input(event));
            handle(&view, &event, &send);
        });
        drop(
            self.textarea
                .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref()),
        );
        listener.forget();
    }
}

/// The textarea as its listeners read it.
struct View {
    textarea: HtmlTextAreaElement,
    shared: Rc<Shared>,
}

/// Out of sight and out of the way: no size, no pointer events.
const HIDDEN: &str = "position: fixed; left: -100px; top: 0; width: 1px; height: 1px; opacity: 0; pointer-events: none;";

/// At the caret, unseen: transparent, 16 px so a phone does not zoom to
/// it, no pointer events so presses reach the canvas.
const SHOWN: &str = "position: fixed; opacity: 0; color: transparent; background: transparent; caret-color: transparent; border: 0; padding: 0; margin: 0; resize: none; overflow: hidden; font-size: 16px; pointer-events: none;";

#[path = "text_bridge_web_view.rs"]
mod view;
