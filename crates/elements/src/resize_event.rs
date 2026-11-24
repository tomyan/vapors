//! A scroll container's `resize` event (virtual list V4,
//! `docs/plans/virtual-list.md`): fired at each scroll container whose
//! laid-out client size differs from the one its last `resize` carried,
//! once per settle, carrying the size (`Event::client_size`) and the
//! container's offset as laid out (`Event::scroll_offset`). The DOM
//! has no element `resize`; a page learns an element's new size from a
//! `ResizeObserver`, whose callbacks run after layout and before paint,
//! and this is modelled on that report, for scroll containers only, the
//! boxes whose window a page may build to (`VirtualList`). Those
//! callbacks run before the browser paints, so `Document::update_frame`
//! fires it after the frame's first update and lays out what its handler
//! changed in the second: the frame that changes a size draws the rows
//! of the new one. A browser repeats that round while each reaches
//! elements deeper in the tree than the last, then reports a loop
//! error; this runs one. It reports the padding box (`clientHeight`),
//! where a default `ResizeObserver` reports the content box, so a
//! change of padding fires one here and none there under
//! `box-sizing: content-box`, and the reverse under `border-box` with a
//! fixed size. A container is
//! observed while it has a `resize` listener: one without owes none, so
//! it never asks for a frame. It does not bubble.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::dispatch_here;
use crate::node::Kind;
use crate::node_map::NodeMap;

/// The client size each observed container's last `resize` carried. A
/// container that has fired none is taken to have reported (0, 0), so a
/// container laid out with any size fires one, as a `ResizeObserver`
/// reports an element when it starts observing it.
#[derive(Debug, Default)]
pub(crate) struct ReportedSizes {
    by_node: NodeMap<[f32; 2]>,
}

impl ReportedSizes {
    /// Whether `size`, `node`'s client size now, differs from the one it
    /// last reported (exact: any change may move a row in or out).
    fn differs(&self, node: NodeId, size: [f32; 2]) -> bool {
        let reported = self.by_node.get(&node).copied().unwrap_or_default();
        reported.map(f32::to_bits) != size.map(f32::to_bits)
    }
}

impl Document {
    /// Fires the `resize` each scroll container owes, in node order,
    /// each carrying the container's offset as last laid out, which a
    /// layout that clamped it changed before its `scroll` could say
    /// (`Document::settle_scroll` calls this after the `scroll`s,
    /// `Document::update_frame` after a frame's first update). Answers
    /// whether it fired any.
    pub(crate) fn settle_resize(document: &Rc<RefCell<Document>>) -> bool {
        let owed = document.borrow_mut().take_owed_resizes();
        let fired = !owed.is_empty();
        for (node, size) in owed {
            // an observed container is a scroll container: it has one
            let offset = document.borrow().scroll_offset(node);
            let offset = offset.expect("a container that owes a resize scrolls");
            let event = Event::new("resize").with_client_size(size);
            dispatch_here(document, node, event.with_scroll_offset(offset));
        }
        fired
    }

    /// Whether any observed scroll container owes a `resize`.
    pub(crate) fn resize_owed(&self) -> bool {
        self.observed_sizes()
            .any(|(node, size)| self.sizes_reported.differs(node, size))
    }

    /// Each scroll container with a `resize` listener, and its client
    /// size as last laid out.
    fn observed_sizes(&self) -> impl Iterator<Item = (NodeId, [f32; 2])> + '_ {
        self.scroll
            .sizes()
            .filter(|(node, _)| self.listens_to_resize(*node))
    }

    /// Whether `node` has a `resize` listener; a node removed since the
    /// update that laid it out has none.
    fn listens_to_resize(&self, node: NodeId) -> bool {
        if !self.arena.contains(node) {
            return false;
        }
        match &self.arena.get(node, "resize").kind {
            Kind::Element(element) => element.listeners.iter().any(|(event, _)| event == "resize"),
            Kind::Text(_) => false,
        }
    }

    /// The observed containers that owe a `resize`, with their sizes, in
    /// node order; every observed container's size is noted as reported,
    /// and one no longer observed or scrolling is forgotten.
    fn take_owed_resizes(&mut self) -> Vec<(NodeId, [f32; 2])> {
        let mut owed: Vec<_> = self
            .observed_sizes()
            .filter(|(node, size)| self.sizes_reported.differs(*node, *size))
            .collect();
        owed.sort_unstable_by_key(|(node, _)| *node);
        self.sizes_reported.by_node = self.observed_sizes().collect();
        owed
    }
}
