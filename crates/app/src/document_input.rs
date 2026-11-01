//! Platform input routed into a [`Document`]: positions from physical to
//! CSS pixels, the pointer, touch gestures and their scrolling, the
//! wheel, keys and typed text, then whether the document changed and
//! the cursor to show. [`ElementsApp`](crate::ElementsApp) routes every
//! event through one; an app that shows a document over its own drawing
//! keeps one for that document and routes the events it gives it (by
//! [`Document::hit`] at [`DocumentInput::at`], say).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Cursor, Document, Point};
use vapors_platform::{Cursor as Shown, Event};

use crate::pointer::action;

/// The input state for one document.
pub struct DocumentInput {
    /// The display scale, for input in physical pixels.
    scale: f32,
    /// Where the pointer last moved, in CSS pixels.
    pointer: Point,
    touch: crate::touch_scroll::TouchScroll,
    wheel: crate::wheel_scroll::WheelScroll,
    /// The cursor last shown.
    cursor: Cursor,
}

/// What an event did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handled {
    /// The document changed: draw a frame.
    pub changed: bool,
    /// The cursor to show, when it is not the one shown.
    pub cursor: Option<Shown>,
}

impl DocumentInput {
    /// Input at display scale `scale`, the pointer at the origin.
    #[must_use]
    pub fn new(scale: f32) -> Self {
        Self {
            scale,
            pointer: [0.0, 0.0],
            touch: crate::touch_scroll::TouchScroll::default(),
            wheel: crate::wheel_scroll::WheelScroll::default(),
            cursor: Cursor::Default,
        }
    }

    /// Take input at `scale` from now: each frame says what it is.
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// Where `event` lands, in CSS pixels: a move, tap or touch at its
    /// position, a button or wheel at where the pointer last moved;
    /// `None` for an event with no place (a key, a pan, the pointer
    /// leaving).
    #[must_use]
    pub fn at(&self, event: &Event) -> Option<Point> {
        let css = |x: f32, y: f32| [x / self.scale, y / self.scale];
        match event {
            Event::PointerMoved { x, y } | Event::Tap { x, y } | Event::Touch { x, y, .. } => {
                Some(css(*x, *y))
            }
            Event::PrimaryButton { .. } | Event::Scroll { .. } => Some(self.pointer),
            _ => None,
        }
    }

    /// Route `event` into `document`.
    pub fn handle(&mut self, document: &Rc<RefCell<Document>>, event: &Event) -> Handled {
        if let Some(action) = action(event, self.scale, self.pointer) {
            self.act(document, action);
        }
        key(document, event);
        let changed = document.borrow_mut().take_changed();
        let cursor = document.borrow().cursor(self.pointer);
        let shown = (cursor != self.cursor).then(|| {
            self.cursor = cursor;
            crate::cursor::shown(cursor)
        });
        Handled {
            changed,
            cursor: shown,
        }
    }
}

/// A key going down or up, to the document's focused element (3-E2a);
/// the text a key produced, to a focused field (3-E3c).
fn key(document: &Rc<RefCell<Document>>, event: &Event) {
    if let Event::Character(text) = event {
        Document::character_typed(document, text);
        return;
    }
    let Event::Key {
        key,
        pressed,
        modifiers,
        ..
    } = event
    else {
        return;
    };
    let modifiers = vapors_elements::Modifiers {
        shift: modifiers.shift,
        control: modifiers.control,
        alt: modifiers.alt,
        meta: modifiers.meta,
    };
    if *pressed {
        Document::key_pressed(document, key, modifiers);
    } else {
        Document::key_released(document, key, modifiers);
    }
}

#[path = "document_input_act.rs"]
mod act;

#[cfg(test)]
#[path = "document_input_tests.rs"]
mod tests;
