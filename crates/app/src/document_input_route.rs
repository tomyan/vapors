//! Which of a document and the app drawing under it an event is for
//! ([`DocumentInput::give`] answers it in [`Given::route`](super::Given::route)),
//! when the app shows the document over its own drawing (touché's card
//! over the photo). The rules, held here so every host keeps them:
//! - every move and the pointer leaving go to the document, so its
//!   hover and cursor stay current and clear when the pointer is over
//!   the host (the host may read moves too);
//! - a press, and each touch-down, go by hit at that moment, and that
//!   route holds for its pointer until the release, or the touch's end
//!   or cancel, so a drag that leaves the document still ends there; a
//!   press or touch-down on the host takes the document's focus away, as
//!   a click outside a page's control blurs it;
//! - a pan goes where its gesture's first touch-down went;
//! - a tap and the wheel go by hit at their own position;
//! - keys and typed text go to the document while one of its elements
//!   has focus, and a key's up goes where its down went (as a press's
//!   release does), so a key held across a focus change still ends where
//!   it began; a key-down the document then does not use is
//!   [`Route::Both`], as are its repeats and its up, so the host's
//!   shortcuts work while a slider has focus and the slider's arrows
//!   still move it;
//! - a paste goes to the document while one of its text fields has focus
//!   (`document_input_paste.rs`), else to the host.
//!
//! A hit is [`Document::hit`]: an element with `pointer-events: auto`.
//! `pointer-events` is inherited, so a document whose `html` and `body`
//! set `none` must set `auto` on the card itself, or nothing is hit.
//! Any element with `auto` takes the pointer over its whole border box,
//! painted or not.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vapors_elements::Document;
use vapors_platform::Event;

use super::{DocumentInput, Unplaced};
use crate::pointer::Action;

/// Who an event is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// The document's alone: [`DocumentInput::give`] gave it.
    Document,
    /// The app drawing under the document takes it; the document did
    /// not have it.
    Host,
    /// Both: the document had it, and the host takes it too. A key-down
    /// the document did not use, its repeats and its up, as a browser
    /// gives the focused element a key and also bubbles it to the page.
    Both,
}

/// The routes held while a press or touch lasts.
#[derive(Default)]
pub(super) struct Held {
    mouse: Option<Route>,
    /// Each key held down, by its place on the keyboard (its DOM `code`,
    /// which Shift does not change as it does `key`), and where its down
    /// went.
    pub(super) keys: HashMap<String, Route>,
    touches: HashMap<u64, Route>,
    gesture: Option<Route>,
}

impl DocumentInput {
    /// Who `event` is for, by the rules above, before the document has
    /// it: [`DocumentInput::give`] asks, then turns a key the document
    /// did not use into [`Route::Both`].
    ///
    /// # Errors
    ///
    /// [`Unplaced`] when the document's last update gives no usable scale.
    pub(crate) fn route(
        &mut self,
        document: &Rc<RefCell<Document>>,
        event: &Event,
    ) -> Result<Route, Unplaced> {
        let route = self.routed(&document.borrow(), event)?;
        let pressed = matches!(event, Event::PrimaryButton { pressed: true, .. })
            || matches!(
                event,
                Event::Touch {
                    phase: vapors_platform::TouchPhase::Started,
                    ..
                }
            );
        if pressed && route == Route::Host && document.borrow().active_element().is_some() {
            Document::focus(document, None);
        }
        Ok(route)
    }

    fn routed(&mut self, document: &Document, event: &Event) -> Result<Route, Unplaced> {
        let Some(action) = self.action(document, event)? else {
            return Ok(self.held.key(document, event));
        };
        if let Action::Move(point) = action {
            // The press after it is routed where the pointer now is.
            self.pointer = point;
        }
        let by_hit = |point| match document.hit(point) {
            Some(_) => Route::Document,
            None => Route::Host,
        };
        Ok(match action {
            Action::Move(_) | Action::Leave => Route::Document,
            Action::Press(p, _) => *self.held.mouse.insert(by_hit(p)),
            Action::Release(p) => self.held.mouse.take().unwrap_or_else(|| by_hit(p)),
            Action::Tap(p) | Action::Wheel(p, _) => by_hit(p),
            Action::TouchDown(id, p) => self.held.touch_down(id, by_hit(p)),
            Action::TouchMove(id, p) => self
                .held
                .touches
                .get(&id)
                .copied()
                .unwrap_or_else(|| by_hit(p)),
            Action::TouchUp(id, p) | Action::TouchCancel(id, p) => {
                self.held.touches.remove(&id).unwrap_or_else(|| by_hit(p))
            }
            Action::Pan(_) => self.held.gesture.unwrap_or(Route::Host),
        })
    }
}

impl Held {
    /// A key's route: a fresh key-down by focus, held by its code until
    /// its key-up; an auto-repeat and the key-up where the key-down
    /// went. Typed text and other placeless events by focus.
    fn key(&mut self, document: &Document, event: &Event) -> Route {
        let Event::Key {
            code,
            pressed,
            repeat,
            ..
        } = event
        else {
            return by_focus(document, event);
        };
        let held = if *pressed && !*repeat {
            None
        } else if *pressed {
            self.keys.get(code).copied()
        } else {
            self.keys.remove(code)
        };
        let route = held.unwrap_or_else(|| by_focus(document, event));
        if *pressed {
            self.keys.insert(code.clone(), route);
        }
        route
    }

    /// A finger down: the first of a gesture sets where its pans go.
    fn touch_down(&mut self, id: u64, route: Route) -> Route {
        if self.touches.is_empty() {
            self.gesture = Some(route);
        }
        self.touches.insert(id, route);
        route
    }
}

/// A key or typed text: the document's while one of its elements has
/// focus; a paste, while one of its text fields has. Any other
/// placeless event is the host's.
fn by_focus(document: &Document, event: &Event) -> Route {
    match event {
        Event::Paste if document.text_field_focused() => Route::Document,
        Event::Key { .. } | Event::Character(_) if document.active_element().is_some() => {
            Route::Document
        }
        _ => Route::Host,
    }
}

#[cfg(test)]
#[path = "document_input_route_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "document_input_keys_tests.rs"]
mod key_tests;

#[cfg(test)]
#[path = "document_input_held_keys_tests.rs"]
mod held_key_tests;
