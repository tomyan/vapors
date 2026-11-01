//! Which of a document and the app drawing under it an event is for,
//! when the app shows the document over its own drawing (touché's card
//! over the photo). The rules, held here so every host keeps them:
//! - every move and the pointer leaving go to the document, so its
//!   hover and cursor stay current and clear when the pointer is over
//!   the host (the host may read moves too);
//! - a press, and each touch-down, go by hit at that moment, and that
//!   route holds for its pointer until the release, or the touch's end
//!   or cancel, so a drag that leaves the document still ends there;
//! - a pan goes where its gesture's first touch-down went;
//! - a tap and the wheel go by hit at their own position;
//! - keys and typed text go to the document while one of its elements
//!   has focus.
//!
//! A hit is [`Document::hit`]: an element with `pointer-events: auto`.
//! `pointer-events` is inherited, so a document whose `html` and `body`
//! set `none` must set `auto` on the card itself, or nothing is hit.
//! Any element with `auto` takes the pointer over its whole border box,
//! painted or not.

use std::collections::HashMap;

use vapors_elements::Document;
use vapors_platform::Event;

use super::{DocumentInput, Unplaced};
use crate::pointer::Action;

/// Who an event is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// Give it to the document ([`DocumentInput::handle`]).
    Document,
    /// The app drawing under the document takes it.
    Host,
}

/// The routes held while a press or touch lasts.
#[derive(Default)]
pub(super) struct Held {
    mouse: Option<Route>,
    touches: HashMap<u64, Route>,
    gesture: Option<Route>,
}

impl DocumentInput {
    /// Who `event` is for, by the rules above. Call it for every event,
    /// in order, before giving the document those routed to it.
    ///
    /// # Errors
    ///
    /// [`Unplaced`] when the document's last update gives no usable scale.
    pub fn route(&mut self, document: &Document, event: &Event) -> Result<Route, Unplaced> {
        let Some(action) = self.action(document, event)? else {
            return Ok(by_focus(document, event));
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
            Action::Press(p) => *self.held.mouse.insert(by_hit(p)),
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
/// focus. Any other placeless event is the host's.
fn by_focus(document: &Document, event: &Event) -> Route {
    match event {
        Event::Key { .. } | Event::Character(_) if document.active_element().is_some() => {
            Route::Document
        }
        _ => Route::Host,
    }
}

#[cfg(test)]
#[path = "document_input_route_tests.rs"]
mod tests;
