//! The runtime behind `{if}` and `{for}`: fragments that own their
//! signals and effects, built before an anchor node and removed as a
//! unit.

mod conditional;
mod keyed;
mod sequence;

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Owner, untrack};

use crate::{Dom, NodeId};

pub use conditional::conditional;
pub use keyed::keyed;

/// A built branch or row: the nodes it placed and the owner of the
/// signals and effects created while building it.
pub struct Fragment {
    owner: Owner,
    /// The top-level nodes placed, in order.
    pub nodes: Vec<NodeId>,
}

impl Fragment {
    /// Builds under a fresh owner (a child of the current one), with
    /// tracking off so signals read while building do not re-run the
    /// block's effect.
    pub fn build<D: Dom>(
        dom: &Rc<RefCell<D>>,
        parent: NodeId,
        before: Option<NodeId>,
        build: impl FnOnce(&Rc<RefCell<D>>, NodeId, Option<NodeId>) -> Vec<NodeId>,
    ) -> Fragment {
        let owner = Owner::new();
        let nodes = owner.with(|| untrack(|| build(dom, parent, before)));
        Fragment { owner, nodes }
    }

    /// Disposes the owner (stopping the effects, running cleanups) and
    /// removes the nodes.
    pub fn remove<D: Dom>(self, dom: &Rc<RefCell<D>>) {
        self.owner.dispose();
        for node in self.nodes {
            dom.borrow_mut().remove(node);
        }
    }
}
