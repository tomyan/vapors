//! A scroll container's `scroll` event (virtual list V1,
//! `docs/plans/virtual-list.md`): fired at each container whose offset
//! differs from the one its last `scroll` carried, once per settle, as
//! Chromium fires an element's `scroll` once per frame in its "run the
//! scroll steps". It does not bubble, as an element's `scroll` does not
//! in the DOM, and carries the offset (`Event::scroll_offset`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::dispatch_here;
use crate::node_map::NodeMap;

/// The offset each container's last `scroll` carried. A container that
/// has fired none is taken to have reported (0, 0), where it starts.
#[derive(Debug, Default)]
pub(crate) struct Reported {
    by_node: NodeMap<[f32; 2]>,
}

impl Reported {
    /// Whether `at`, `node`'s offset now, differs from the one it last
    /// reported (exact: any change is one a listener may show).
    fn differs(&self, node: NodeId, at: [f32; 2]) -> bool {
        let reported = self.by_node.get(&node).copied().unwrap_or_default();
        reported.map(f32::to_bits) != at.map(f32::to_bits)
    }
}

impl Document {
    /// Fires the `scroll` each scroll container owes: one at each whose
    /// offset (as [`Document::scroll_offset`] answers it) changed since
    /// its last `scroll`, by any route (wheel, touch, fling,
    /// `scroll_to`, a press's scroll into view, or an update clamping it
    /// to content that shrank), in node order; then the `resize` each
    /// owes, at each whose client size the last update changed
    /// (`resize_event.rs`). `Document::update_frame` calls this before
    /// the frame's first update and again after it, so what a listener
    /// changes is drawn in that frame.
    pub fn settle_scroll(document: &Rc<RefCell<Document>>) {
        let owed = document.borrow_mut().take_owed_scrolls();
        for (node, offset) in owed {
            let event = Event::new("scroll").with_scroll_offset(offset);
            dispatch_here(document, node, event);
        }
        Document::settle_resize(document);
    }

    /// Whether any scroll container owes a `scroll`
    /// ([`Document::settle_scroll`]).
    pub(crate) fn scroll_owed(&self) -> bool {
        self.scroll
            .each()
            .any(|(node, at)| self.scroll_reported.differs(node, at))
    }

    /// The containers that owe a `scroll`, with their offsets, in node
    /// order; every container's offset is noted as reported, and a
    /// container that no longer scrolls is forgotten.
    fn take_owed_scrolls(&mut self) -> Vec<(NodeId, [f32; 2])> {
        let mut owed: Vec<_> = self
            .scroll
            .each()
            .filter(|(node, at)| self.scroll_reported.differs(*node, *at))
            .collect();
        owed.sort_unstable_by_key(|(node, _)| *node);
        self.scroll_reported.by_node = self.scroll.each().collect();
        owed
    }
}
