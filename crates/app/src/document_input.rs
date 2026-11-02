//! Platform input routed into a [`Document`]: positions from physical to
//! CSS pixels at the scale of the document's last update, the pointer,
//! touch gestures and their scrolling, the wheel, keys and typed text,
//! then whether the document changed and the cursor to show.
//! An app that draws a document over its own drawing gives every event to
//! [`DocumentInput::give`], which routes it (`document_input_route.rs`),
//! gives the document what is its, a paste included
//! (`document_input_paste.rs`), and says who the event was for in the end.
//! [`ElementsApp`](crate::ElementsApp), whose document is the whole window,
//! gives it everything.
//!
//! Single-threaded, as `Document`'s own event calls are: it takes the
//! document as `Rc<RefCell<Document>>` (touché
//! docs/decisions/2026-10-28-keep-native-fast-path-open.md, item 5).
//! That would retire if `vapors-elements` took events by `&mut Document`.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Cursor, Document, Point};
use vapors_platform::{Event, Platform};

use crate::pointer::{Action, action};

/// The input state for one document.
pub struct DocumentInput {
    /// Where the pointer last moved, in CSS pixels.
    pointer: Point,
    touch: crate::touch_scroll::TouchScroll,
    wheel: crate::wheel_scroll::WheelScroll,
    /// The cursor last reported; `None` once the host showed its own.
    cursor: Option<Cursor>,
    /// Which side each press and touch went to, until it ends.
    held: route::Held,
    /// The clipboard a paste into a text field reads, and the reads
    /// started, answered off the loop.
    #[cfg(not(target_family = "wasm"))]
    pastes: paste::Pastes,
}

impl Default for DocumentInput {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentInput {
    /// No pointer yet, nothing held.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pointer: [0.0, 0.0],
            touch: crate::touch_scroll::TouchScroll::default(),
            wheel: crate::wheel_scroll::WheelScroll::default(),
            cursor: Some(Cursor::Default),
            held: route::Held::default(),
            #[cfg(not(target_family = "wasm"))]
            pastes: paste::Pastes::default(),
        }
    }

    /// Where `event` lands in CSS pixels, as the document is given it: a
    /// move, tap or touch at its position, a button or wheel where the
    /// pointer last moved; `None` for an event with no place (a key, a
    /// pan, the pointer leaving).
    ///
    /// # Errors
    ///
    /// [`Unplaced`] when the document's last update gives no usable scale.
    pub fn at(&self, document: &Document, event: &Event) -> Result<Option<Point>, Unplaced> {
        Ok(self.action(document, event)?.and_then(point_of))
    }

    /// The host has shown a cursor of its own: the document's is
    /// reported again on the next event.
    pub fn forget_cursor(&mut self) {
        self.cursor = None;
    }

    /// Route `event` (`document_input_route.rs`), give the document what
    /// is its, and say who the event was for in the end: the host acts on
    /// it exactly when [`Given::route`] is [`Route::Host`] or
    /// [`Route::Both`]. A paste routed to the document is read from the
    /// clipboard here (native only; see `document_input_paste.rs`).
    ///
    /// # Errors
    ///
    /// [`Unplaced`], and the event is not given, when it is a pointer
    /// event and the document's last update gives no usable scale. A key
    /// or typed text needs no scale and is always given.
    pub fn give(
        &mut self,
        document: &Rc<RefCell<Document>>,
        event: &Event,
        platform: &Platform,
    ) -> Result<Given, Unplaced> {
        #[cfg(not(target_family = "wasm"))]
        self.deliver_pastes(document);
        let route = self.route(document, event)?;
        if route == Route::Host {
            return Ok(Given {
                route,
                changed: false,
                cursor: None,
            });
        }
        let handled = self.handle(document, event, platform)?;
        let left = route == Route::Document && handled.unused_key;
        Ok(Given {
            route: if left { Route::Both } else { route },
            changed: handled.changed,
            cursor: handled.cursor,
        })
    }

    /// Hand `event` to `document`, whatever its route: `ElementsApp`'s
    /// document is the whole window.
    pub(crate) fn handle(
        &mut self,
        document: &Rc<RefCell<Document>>,
        event: &Event,
        platform: &Platform,
    ) -> Result<Handled, Unplaced> {
        let action = self.action(&document.borrow(), event)?;
        if let Some(action) = action {
            self.act(document, action);
        }
        if let Event::WindowFocus { focused } = event {
            self.window_focus(document, *focused);
        }
        let unused_key = key(document, event) == Some(false)
            && !keys::field_paste_key(&document.borrow(), event);
        if let (true, Event::Key { code, .. }) = (unused_key, event) {
            // the document had its down and left it to the host: its
            // repeats and its up go to both
            self.held.keys.insert(code.clone(), Route::Both);
        }
        self.paste_if(document, event, platform);
        let changed = document.borrow_mut().take_changed();
        let cursor = document.borrow().cursor(self.pointer);
        let shown = (Some(cursor) != self.cursor).then(|| {
            self.cursor = Some(cursor);
            crate::cursor::shown(cursor)
        });
        Ok(Handled {
            changed,
            cursor: shown,
            unused_key,
        })
    }

    /// The pointer action for `event` at the document's scale. The
    /// scale is read only for an event that is a pointer action (any
    /// scale tells which), so a key or typed text is delivered by focus
    /// even before the first update.
    fn action(&self, document: &Document, event: &Event) -> Result<Option<Action>, Unplaced> {
        if action(event, 1.0, self.pointer).is_none() {
            return Ok(None);
        }
        Ok(action(event, scale_of(document)?, self.pointer))
    }
}

/// The scale of `document`'s last update, refused when there is none or
/// it is not a positive number.
fn scale_of(document: &Document) -> Result<f32, Unplaced> {
    let scale = document.painted().ok_or(Unplaced::NotUpdated)?.scale();
    if scale.is_finite() && scale > 0.0 {
        Ok(scale)
    } else {
        Err(Unplaced::Scale(scale))
    }
}

/// The CSS-pixel point an action is at, if it has one.
fn point_of(action: Action) -> Option<Point> {
    match action {
        Action::Move(p)
        | Action::Press(p, _, _)
        | Action::Release(p, _)
        | Action::Tap(p, _)
        | Action::DoubleTap(p)
        | Action::Wheel(p, _)
        | Action::TouchDown(_, p)
        | Action::TouchMove(_, p)
        | Action::TouchUp(_, p)
        | Action::TouchCancel(_, p) => Some(p),
        Action::Leave | Action::Pan(_) => None,
    }
}

#[path = "document_input_act.rs"]
mod act;
#[path = "document_input_keys.rs"]
mod keys;
use keys::key;
#[cfg(not(target_family = "wasm"))]
#[path = "document_input_paste.rs"]
mod paste;
#[cfg(target_family = "wasm")]
impl DocumentInput {
    /// On the web the platform sends no `Event::Paste`: RELAXED, see
    /// `document_input_paste.rs`.
    #[allow(clippy::unused_self)] // the native paste's signature, so `handle` is one code path
    fn paste_if(&mut self, _: &Rc<RefCell<Document>>, _: &Event, _: &Platform) {}
}
#[path = "document_input_route.rs"]
mod route;
pub use route::Route;
#[cfg(test)]
#[path = "document_input_click_tests.rs"]
mod click_tests;
#[path = "document_input_types.rs"]
mod types;
pub(crate) use types::Handled;
pub use types::{Given, Unplaced};

#[cfg(test)]
#[path = "document_input_tests.rs"]
mod tests;
