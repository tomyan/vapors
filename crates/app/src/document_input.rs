//! Platform input routed into a [`Document`]: positions from physical to
//! CSS pixels at the scale of the document's last update, the pointer,
//! touch gestures and their scrolling, the wheel, keys and typed text,
//! then whether the document changed and the cursor to show.
//! [`ElementsApp`](crate::ElementsApp) gives every event to one. An app
//! that draws a document over its own drawing asks [`DocumentInput::route`]
//! which of the two each event is for (`document_input_route.rs`).
//!
//! Single-threaded, as `Document`'s own event calls are: it takes the
//! document as `Rc<RefCell<Document>>` (touché
//! docs/decisions/2026-10-28-keep-native-fast-path-open.md, item 5).
//! That would retire if `vapors-elements` took events by `&mut Document`.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use vapors_elements::{Cursor, Document, Point};
use vapors_platform::{Cursor as Shown, Event};

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
}

/// What an event did to the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputOutcome {
    /// The document changed: draw a frame.
    pub changed: bool,
    /// The cursor to show, when it is not the one last reported.
    pub cursor: Option<Shown>,
    /// A key went down that the document did not use (no listener
    /// prevented it, no control took it): the host's to act on.
    pub unused_key: bool,
}

/// Why an event could not be placed in the document.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unplaced {
    /// The document has not been updated, so nothing is on screen and
    /// no position can be converted.
    NotUpdated,
    /// The last update's scale is zero, negative or not a number.
    Scale(f32),
}

impl fmt::Display for Unplaced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUpdated => write!(
                f,
                "the document has not been updated, so input has nowhere to land"
            ),
            Self::Scale(scale) => write!(
                f,
                "the document was updated at scale {scale}, which is not a positive number; pass the display's device pixels per CSS pixel in its Viewport"
            ),
        }
    }
}

impl std::error::Error for Unplaced {}

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

    /// Give `event` to `document`.
    ///
    /// # Errors
    ///
    /// [`Unplaced`], and the event is not given, when it is a pointer
    /// event and the document's last update gives no usable scale. A key
    /// or typed text needs no scale and is always given.
    pub fn handle(
        &mut self,
        document: &Rc<RefCell<Document>>,
        event: &Event,
    ) -> Result<InputOutcome, Unplaced> {
        let action = self.action(&document.borrow(), event)?;
        if let Some(action) = action {
            self.act(document, action);
        }
        let unused_key = key(document, event) == Some(false);
        let changed = document.borrow_mut().take_changed();
        let cursor = document.borrow().cursor(self.pointer);
        let shown = (Some(cursor) != self.cursor).then(|| {
            self.cursor = Some(cursor);
            crate::cursor::shown(cursor)
        });
        Ok(InputOutcome {
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
        | Action::Press(p)
        | Action::Release(p)
        | Action::Tap(p)
        | Action::Wheel(p, _)
        | Action::TouchDown(_, p)
        | Action::TouchMove(_, p)
        | Action::TouchUp(_, p)
        | Action::TouchCancel(_, p) => Some(p),
        Action::Leave | Action::Pan(_) => None,
    }
}

/// A key going down or up, to the document's focused element (3-E2a);
/// the text a key produced, to a focused field (3-E3c). For a key going
/// down, whether the document used it.
fn key(document: &Rc<RefCell<Document>>, event: &Event) -> Option<bool> {
    if let Event::Character(text) = event {
        Document::character_typed(document, text);
        return None;
    }
    let Event::Key {
        key,
        pressed,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    let modifiers = vapors_elements::Modifiers {
        shift: modifiers.shift,
        control: modifiers.control,
        alt: modifiers.alt,
        meta: modifiers.meta,
    };
    if *pressed {
        Some(Document::key_pressed(document, key, modifiers))
    } else {
        Document::key_released(document, key, modifiers);
        None
    }
}

#[path = "document_input_act.rs"]
mod act;
#[path = "document_input_route.rs"]
mod route;
pub use route::Route;

#[cfg(test)]
#[path = "document_input_tests.rs"]
mod tests;
