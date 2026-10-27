//! The tree walks pointer input needs: the chain from a node to the
//! root, the flags set along it, the nearest common ancestor, and the
//! dispatch itself.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_css::tree::States;
use vapors_dom::{Capture, Event, Handler, NodeId};

use crate::document::Document;
use crate::node::Kind;

impl Document {
    /// `node` and its ancestors, the node first.
    pub(crate) fn chain(&self, node: NodeId) -> Vec<NodeId> {
        let mut chain = vec![node];
        while let Some(parent) = self.parent(*chain.last().expect("never empty")) {
            chain.push(parent);
        }
        chain
    }

    /// Sets `flag` on `target` and its ancestors and clears it on every
    /// other element, marking the document changed when any flag moved.
    pub(crate) fn set_flag_chain(&mut self, flag: States, target: Option<NodeId>) {
        let chain = target.map(|node| self.chain(node)).unwrap_or_default();
        self.set_flag_only(flag, &chain);
    }

    /// Sets `flag` on `target` alone and clears it on every other element
    /// (`:focus`, which its ancestors do not share).
    pub(crate) fn set_flag_on(&mut self, flag: States, target: Option<NodeId>) {
        self.set_flag_only(flag, &target.into_iter().collect::<Vec<_>>());
    }

    /// Sets `flag` on the elements of `chain` and clears it on every
    /// other, marking the document changed when any flag moved.
    fn set_flag_only(&mut self, flag: States, chain: &[NodeId]) {
        for node in self.elements() {
            let Kind::Element(element) = &mut self.arena.get_mut(node, "set_flag_chain").kind
            else {
                continue;
            };
            let wanted = chain.contains(&node);
            if element.states.contains(flag) != wanted {
                element.states = if wanted {
                    element.states.union(flag)
                } else {
                    element.states.without(flag)
                };
                self.changed = true;
                self.mark_flag(node, flag);
            }
        }
    }

    /// The nearest element that is, or holds, both `a` and `b`.
    pub(crate) fn common_ancestor(&self, a: NodeId, b: NodeId) -> Option<NodeId> {
        let of_b = self.chain(b);
        self.chain(a).into_iter().find(|node| of_b.contains(node))
    }

    /// Whether `node` is, or is inside, a `<button disabled>`, or is an
    /// `<input disabled>`: neither is clicked (3-E3a: Chromium sends a
    /// disabled input its `pointerdown` and `pointerup`, not the click).
    pub(crate) fn in_disabled_button(&self, node: NodeId) -> bool {
        self.chain(node).into_iter().any(|n| {
            self.arena
                .get(n, "in_disabled_button")
                .element()
                .is_some_and(|e| {
                    matches!(e.tag.as_str(), "button" | "input")
                        && e.attributes.contains_key("disabled")
                })
        })
    }

    /// The listeners for `name` on each node of `target`'s chain, target
    /// first, with their node, cloned so none is called with the
    /// document borrowed.
    fn listeners_along(
        &self,
        target: NodeId,
        name: &str,
    ) -> Vec<(NodeId, Vec<Rc<RefCell<Handler>>>)> {
        self.chain(target)
            .into_iter()
            .map(|node| match &self.arena.get(node, "dispatch").kind {
                Kind::Element(element) => (
                    node,
                    element
                        .listeners
                        .iter()
                        .filter(|(event, _)| event == name)
                        .map(|(_, handler)| Rc::clone(handler))
                        .collect(),
                ),
                Kind::Text(_) => (node, Vec::new()),
            })
            .collect()
    }
}

/// Delivers `event` to `target`, then each ancestor, until a listener
/// stops propagation; what listeners asked of the pointer's capture,
/// with the element each listener is on.
pub(super) fn dispatch(
    document: &Rc<RefCell<Document>>,
    target: NodeId,
    event: Event,
) -> Vec<(NodeId, Capture)> {
    deliver(document, target, event, true).1
}

/// Delivers `event` to `target`, and when it `bubbles` to each ancestor
/// until a listener stops propagation; the event as the listeners left
/// it, and what they asked of the pointer's capture.
pub(crate) fn deliver(
    document: &Rc<RefCell<Document>>,
    target: NodeId,
    mut event: Event,
    bubbles: bool,
) -> (Event, Vec<(NodeId, Capture)>) {
    event = event.at(target);
    let mut along = document.borrow().listeners_along(target, event.name());
    if !bubbles {
        along.truncate(1);
    }
    let mut asked = Vec::new();
    for (node, listeners) in along {
        for listener in listeners {
            (listener.borrow_mut())(&mut event);
            asked.extend(event.take_capture().map(|capture| (node, capture)));
        }
        if event.propagation_stopped() {
            break;
        }
    }
    (event, asked)
}

/// `event` at `node` only: an event that does not bubble (`focus`).
pub(crate) fn dispatch_here(document: &Rc<RefCell<Document>>, node: NodeId, event: Event) {
    deliver(document, node, event, false);
}

/// `event` at `node` and up its ancestors; whether a listener prevented
/// its default action.
pub(crate) fn dispatch_bubbling(
    document: &Rc<RefCell<Document>>,
    node: NodeId,
    event: Event,
) -> bool {
    deliver(document, node, event, true).0.default_prevented()
}
