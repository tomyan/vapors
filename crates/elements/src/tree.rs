//! Linking nodes: placing a child, detaching it, walking children.
//! Every check that the generated code called the tree correctly is
//! here, and each failure panics naming the call and the ids.

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::node::Kind;

impl Arena {
    /// Places `child` in `parent` before `before` (last when `None`),
    /// detaching it first if it is placed somewhere already.
    pub(crate) fn place(
        &mut self,
        call: &str,
        parent: NodeId,
        child: NodeId,
        before: Option<NodeId>,
    ) {
        self.check_can_hold(call, parent, child, before);
        if before == Some(child) {
            return; // inserting a node before itself leaves it where it is, as in the DOM
        }
        self.detach(child);
        let previous = match before {
            Some(before) => self.get(before, call).previous,
            None => self.get(parent, call).last_child,
        };
        self.link(parent, child, previous, before);
    }

    fn link(
        &mut self,
        parent: NodeId,
        child: NodeId,
        previous: Option<NodeId>,
        next: Option<NodeId>,
    ) {
        let node = self.get_mut(child, "link");
        node.parent = Some(parent);
        node.previous = previous;
        node.next = next;
        match previous {
            Some(previous) => self.get_mut(previous, "link").next = Some(child),
            None => self.get_mut(parent, "link").first_child = Some(child),
        }
        match next {
            Some(next) => self.get_mut(next, "link").previous = Some(child),
            None => self.get_mut(parent, "link").last_child = Some(child),
        }
    }

    /// Unlinks `node` from its parent and siblings; its subtree stays
    /// attached to it.
    pub(crate) fn detach(&mut self, node: NodeId) {
        let (parent, previous, next) = {
            let n = self.get(node, "detach");
            (n.parent, n.previous, n.next)
        };
        let Some(parent) = parent else { return };
        match previous {
            Some(previous) => self.get_mut(previous, "detach").next = next,
            None => self.get_mut(parent, "detach").first_child = next,
        }
        match next {
            Some(next) => self.get_mut(next, "detach").previous = previous,
            None => self.get_mut(parent, "detach").last_child = previous,
        }
        let n = self.get_mut(node, "detach");
        (n.parent, n.previous, n.next) = (None, None, None);
    }

    /// Frees `node` and every descendant.
    pub(crate) fn free_subtree(&mut self, node: NodeId) {
        for child in self.children(node) {
            self.free_subtree(child);
        }
        self.free(node);
    }

    /// The children of `node`, in order.
    pub(crate) fn children(&self, node: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut next = self.get(node, "children").first_child;
        while let Some(child) = next {
            out.push(child);
            next = self.get(child, "children").next;
        }
        out
    }

    fn check_can_hold(&self, call: &str, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        if let Kind::Text(text) = &self.get(parent, call).kind {
            panic!(
                "{call}: parent {parent:?} is a text node ({text:?}); only elements hold children"
            );
        }
        self.get(child, call);
        if let Some(before) = before {
            let actual = self.get(before, call).parent;
            assert_eq!(
                actual,
                Some(parent),
                "{call}: `before` {before:?} is a child of {actual:?}, not of the parent {parent:?}"
            );
        }
        self.check_not_ancestor(call, parent, child);
    }

    fn check_not_ancestor(&self, call: &str, parent: NodeId, child: NodeId) {
        let mut current = Some(parent);
        while let Some(node) = current {
            assert_ne!(
                node, child,
                "{call}: placing {child:?} inside {parent:?} would make a cycle; {child:?} is {parent:?} or one of its ancestors"
            );
            current = self.get(node, call).parent;
        }
    }
}
