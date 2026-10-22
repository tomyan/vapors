//! The runtime behind `{if}` and `{for}`: fragments that own their
//! signals and effects, built before an anchor node and removed as a
//! unit.

mod conditional;
mod keyed;
mod sequence;

use std::cell::RefCell;
use std::rc::Rc;

use vapors_signals::{Owner, untrack};

use crate::{Dom, NodeId, Root, current_nodes};

pub use conditional::conditional;
pub use keyed::keyed;

/// A built branch or row: what it placed at its top level and the
/// owner of the signals and effects created while building it.
pub struct Fragment {
    owner: Owner,
    roots: Vec<Root>,
}

impl Fragment {
    /// Builds under a fresh owner (a child of the current one), with
    /// tracking off so signals read while building do not re-run the
    /// block's effect.
    pub fn build<D: Dom>(
        dom: &Rc<RefCell<D>>,
        parent: NodeId,
        before: Option<NodeId>,
        build: impl FnOnce(&Rc<RefCell<D>>, NodeId, Option<NodeId>) -> Vec<Root>,
    ) -> Fragment {
        let owner = Owner::new();
        let roots = owner.with(|| untrack(|| build(dom, parent, before)));
        Fragment { owner, roots }
    }

    /// The top-level nodes now, nested blocks' included, in order.
    #[must_use]
    pub fn nodes(&self) -> Vec<NodeId> {
        current_nodes(&self.roots)
    }

    /// Reads the nodes, then disposes the owner (stopping the effects,
    /// running cleanups) and removes them.
    pub fn remove<D: Dom>(self, dom: &Rc<RefCell<D>>) {
        let nodes = self.nodes();
        self.owner.dispose();
        for node in nodes {
            dom.borrow_mut().remove(node);
        }
    }
}
