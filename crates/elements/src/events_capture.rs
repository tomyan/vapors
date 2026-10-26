//! Pointer capture (3-E1a, Pointer Events 3 section 10): a listener's
//! `set_pointer_capture` makes its element the target of the pointer's
//! later events wherever the pointer is, until `pointerup` or
//! `release_pointer_capture`. As the specification orders it, a change
//! takes effect before the pointer's next event, with
//! `lostpointercapture` to the element that had it and
//! `gotpointercapture` to the one that gets it; `pointerup` ends it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Capture, Event, NodeId, Pointer};

use super::{Asked, Point, tree::dispatch};
use crate::document::Document;

/// The mouse's pointer id; touches have their own (3-E1b).
const MOUSE: u32 = 1;

impl Document {
    /// Where the pointer's events at `point` go: the element capturing
    /// it, while it is in the document, or else the one `point` hits.
    pub(super) fn pointer_target(&self, point: Point) -> Option<NodeId> {
        let capturing = self.pointer.capture.filter(|node| self.contains(*node));
        capturing.or_else(|| self.hit(point))
    }

    /// `name` at `point`, with the offset from `target`'s padding box.
    pub(super) fn pointer_event(&self, name: &str, target: NodeId, point: Point) -> Event {
        let origin = self
            .painted
            .as_ref()
            .and_then(|painted| painted.padding_origin(target))
            .unwrap_or([0.0, 0.0]);
        let offset = [point[0] - origin[0], point[1] - origin[1]];
        let pressed =
            matches!(name, "pointerdown" | "pointermove") && self.pointer.pressed.is_some();
        Event::new(name).with_pointer(Pointer {
            client: point,
            offset,
            id: MOUSE,
            pressed,
        })
    }
}

/// Dispatches a pointer event and keeps what its listeners asked of the
/// capture, for the next event.
pub(super) fn dispatch_pointer(document: &Rc<RefCell<Document>>, target: NodeId, event: Event) {
    for (node, asked) in dispatch(document, target, event) {
        document.borrow_mut().pointer.pending = Some(match asked {
            Capture::Set => Asked::CaptureTo(node),
            Capture::Release => Asked::Release,
        });
    }
}

/// Asks for the capture to end, as `pointerup` does.
pub(super) fn release_capture(document: &Rc<RefCell<Document>>) {
    document.borrow_mut().pointer.pending = Some(Asked::Release);
}

/// Makes an asked-for change of capture: `lostpointercapture` to the
/// element that had it, then `gotpointercapture` to the one that gets it.
pub(super) fn process_pending(document: &Rc<RefCell<Document>>, point: Point) {
    let (had, gets) = {
        let mut doc = document.borrow_mut();
        let gets = match doc.pointer.pending.take() {
            None => return,
            Some(Asked::CaptureTo(node)) => Some(node),
            Some(Asked::Release) => None,
        };
        let had = std::mem::replace(&mut doc.pointer.capture, gets);
        // an element removed from the document hears nothing: the web
        // fires on it, where nothing can observe it (review-v77, H1)
        let present = |node: Option<NodeId>| node.filter(|node| doc.contains(*node));
        (present(had), present(gets))
    };
    if had == gets {
        return;
    }
    for (name, node) in [("lostpointercapture", had), ("gotpointercapture", gets)] {
        if let Some(node) = node {
            let event = document.borrow().pointer_event(name, node, point);
            dispatch(document, node, event);
        }
    }
}

/// While the pointer is captured, `:hover` stays on the capturing
/// element, as on the web (review-v77, M1); `None` when not captured.
pub(super) fn hover_capturing(document: &Rc<RefCell<Document>>) -> Option<NodeId> {
    let mut doc = document.borrow_mut();
    let capturing = doc.pointer.capture.filter(|node| doc.contains(*node))?;
    if doc.pointer.hovered != Some(capturing) {
        doc.set_flag_chain(vapors_css::tree::States::HOVER, Some(capturing));
        doc.pointer.hovered = Some(capturing);
    }
    Some(capturing)
}
