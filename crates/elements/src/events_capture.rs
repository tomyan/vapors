//! Pointer capture (3-E1, Pointer Events 3 section 10), per pointer: a
//! listener's `set_pointer_capture` makes its element the target of the
//! pointer's later events wherever the pointer is, until `pointerup`,
//! `pointercancel` or `release_pointer_capture`. A touch is captured by
//! its `pointerdown` target without asking (implicit capture). As the
//! specification orders it, a change takes effect before the pointer's
//! next event, with `lostpointercapture` to the element that had it and
//! `gotpointercapture` to the one that gets it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Capture, Event, NodeId, Pointer};

use super::{Asked, Point, tree::dispatch};
use crate::document::Document;

/// The mouse's pointer id; touch `n` is `TOUCHES + n`.
pub(crate) const MOUSE: u32 = 1;
pub(crate) const TOUCHES: u32 = 2;

/// One pointer's capture.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Captured {
    pub(crate) capture: Option<NodeId>,
    pub(crate) pending: Option<Asked>,
    /// A listener asked for the capture, rather than a touch's implicit one.
    pub(crate) explicit: bool,
}

impl Document {
    /// Where pointer `id`'s events at `point` go: the element capturing
    /// it, while it is in the document, or else the one `point` hits.
    pub(super) fn pointer_target(&self, id: u32, point: Point) -> Option<NodeId> {
        let capturing = self
            .captured(id)
            .capture
            .filter(|node| self.contains(*node));
        capturing.or_else(|| self.hit(point))
    }

    pub(super) fn captured(&self, id: u32) -> Captured {
        self.pointer.captures.get(&id).copied().unwrap_or_default()
    }

    /// `name` for pointer `id` at `point`, with the offset from
    /// `target`'s padding box.
    pub(super) fn pointer_event(&self, name: &str, id: u32, target: NodeId, point: Point) -> Event {
        let origin = self
            .painted
            .as_ref()
            .and_then(|painted| painted.padding_origin(target))
            .unwrap_or([0.0, 0.0]);
        let offset = [point[0] - origin[0], point[1] - origin[1]];
        // a touch is pressed while it is down; the mouse while its button is
        let down = if id == MOUSE {
            self.pointer.pressed.is_some()
        } else {
            true
        };
        let pressed = matches!(name, "pointerdown" | "pointermove") && down;
        Event::new(name).with_pointer(Pointer {
            client: point,
            offset,
            id,
            pressed,
        })
    }
}

/// Dispatches pointer `id`'s event and keeps what its listeners asked
/// of the capture, for the pointer's next event; whether a listener
/// prevented its default action.
pub(super) fn dispatch_pointer(
    document: &Rc<RefCell<Document>>,
    id: u32,
    target: NodeId,
    event: Event,
) -> bool {
    let (event, asked) = super::tree::deliver(document, target, event, true);
    for (node, asked) in asked {
        let mut doc = document.borrow_mut();
        let captured = doc.pointer.captures.entry(id).or_default();
        captured.pending = Some(match asked {
            Capture::Set => Asked::CaptureTo(node),
            Capture::Release => Asked::Release,
        });
        captured.explicit = asked == Capture::Set;
    }
    event.default_prevented()
}

/// Asks for pointer `id`'s capture to go to `asked`, unless a listener
/// already asked for something (a touch's implicit capture yields to it).
pub(crate) fn ask_unless_asked(document: &Rc<RefCell<Document>>, id: u32, asked: Asked) {
    let mut doc = document.borrow_mut();
    let captured = doc.pointer.captures.entry(id).or_default();
    captured.pending.get_or_insert(asked);
}

/// Ends pointer `id`'s capture, as `pointerup` and `pointercancel` do.
pub(super) fn release_capture(document: &Rc<RefCell<Document>>, id: u32) {
    let mut doc = document.borrow_mut();
    let captured = doc.pointer.captures.entry(id).or_default();
    captured.pending = Some(Asked::Release);
    captured.explicit = false;
}

/// Makes an asked-for change of pointer `id`'s capture:
/// `lostpointercapture` to the element that had it, then
/// `gotpointercapture` to the one that gets it.
pub(super) fn process_pending(document: &Rc<RefCell<Document>>, id: u32, point: Point) {
    let (had, gets) = {
        let mut doc = document.borrow_mut();
        let captured = doc.pointer.captures.entry(id).or_default();
        let gets = match captured.pending.take() {
            None => return,
            Some(Asked::CaptureTo(node)) => Some(node),
            Some(Asked::Release) => None,
        };
        let had = std::mem::replace(&mut captured.capture, gets);
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
            let event = document.borrow().pointer_event(name, id, node, point);
            dispatch(document, node, event);
        }
    }
}

/// While the pointer is captured, `:hover` stays on the capturing
/// element, as on the web (review-v77, M1); `None` when not captured.
pub(super) fn hover_capturing(document: &Rc<RefCell<Document>>) -> Option<NodeId> {
    let mut doc = document.borrow_mut();
    let capturing = doc
        .captured(MOUSE)
        .capture
        .filter(|node| doc.contains(*node))?;
    if doc.pointer.hovered != Some(capturing) {
        doc.set_flag_chain(vapors_css::tree::States::HOVER, Some(capturing));
        doc.pointer.hovered = Some(capturing);
    }
    Some(capturing)
}
