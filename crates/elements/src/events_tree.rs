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

    /// Whether `node` is, or is inside, a `<button disabled>`.
    pub(crate) fn in_disabled_button(&self, node: NodeId) -> bool {
        self.chain(node).into_iter().any(|n| {
            self.arena
                .get(n, "in_disabled_button")
                .element()
                .is_some_and(|e| e.tag == "button" && e.attributes.contains_key("disabled"))
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
    mut event: Event,
) -> Vec<(NodeId, Capture)> {
    let along = document.borrow().listeners_along(target, event.name());
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
    asked
}
