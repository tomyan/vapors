//! `Effect`: a side effect that runs once when created and again,
//! after the write or the outermost batch, whenever something it read
//! changes.

use std::cell::RefCell;
use std::fmt;
use std::panic::Location;
use std::rc::Rc;

use crate::error::{Kind, NodeInfo};
use crate::node::{NodeId, State};
use crate::owner::dispose_node;
use crate::propagate::recompute;
use crate::runtime::with_runtime;

/// A handle to a running effect. Dropping the handle does not stop the
/// effect; its owner, or [`Effect::dispose`], does.
#[derive(Clone, Copy)]
pub struct Effect {
    id: NodeId,
    info: NodeInfo,
}

impl fmt::Debug for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info)
    }
}

impl Effect {
    /// Creates an effect owned by the current owner and runs it once
    /// now. Each run first disposes what the previous run created and
    /// runs its cleanups.
    #[track_caller]
    pub fn new(f: impl FnMut() + 'static) -> Effect {
        Self::create(None, f, Location::caller())
    }

    /// Creates an effect that messages name as `name`.
    #[track_caller]
    pub fn named(name: &'static str, f: impl FnMut() + 'static) -> Effect {
        Self::create(Some(name), f, Location::caller())
    }

    fn create(
        name: Option<&'static str>,
        mut f: impl FnMut() + 'static,
        created_at: &'static Location<'static>,
    ) -> Effect {
        let (id, info) = with_runtime(|runtime| {
            let id = runtime.create_node(Kind::Effect, name, created_at, State::Dirty);
            let scope = runtime.create_owner();
            let node = runtime.node_mut(id).expect("just created");
            node.compute = Some(Rc::new(RefCell::new(move || {
                f();
                true
            })));
            node.scope = Some(scope);
            (id, node.info)
        });
        recompute(id);
        Effect { id, info }
    }

    /// Kind, name and creation site, for messages.
    #[must_use]
    pub fn info(&self) -> NodeInfo {
        self.info
    }

    /// Stops the effect, running the cleanups of its last run.
    pub fn dispose(self) {
        dispose_node(self.id);
    }
}
