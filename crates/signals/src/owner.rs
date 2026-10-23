//! Owners: the scopes that dispose signals, memos, effects and cleanups
//! together.

use crate::guard::Restore;
use crate::node::{Cleanup, Compute, NodeId, OwnerId, ValueCell};
use crate::runtime::{Runtime, with_runtime};

/// A scope that owns the nodes created while it is current. Disposing
/// it disposes its child owners, its nodes and runs its cleanups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owner {
    id: OwnerId,
}

impl Default for Owner {
    fn default() -> Self {
        Self::new()
    }
}

impl Owner {
    /// Creates an owner as a child of the current one (or a root).
    pub fn new() -> Owner {
        Owner {
            id: with_runtime(Runtime::create_owner),
        }
    }

    /// The owner nodes are currently created under, if any.
    #[must_use]
    pub fn current() -> Option<Owner> {
        with_runtime(|runtime| runtime.current_owner).map(|id| Owner { id })
    }

    /// Makes this owner current until another `set` or `with` changes it.
    pub fn set(&self) {
        with_runtime(|runtime| runtime.current_owner = Some(self.id));
    }

    /// Runs `f` with this owner current, then restores the previous one.
    pub fn with<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = with_runtime(|runtime| runtime.current_owner.replace(self.id));
        let _restore =
            Restore::new(move || with_runtime(|runtime| runtime.current_owner = previous));
        f()
    }

    /// Disposes the owner and everything it owns. A disposed owner's
    /// signals refuse reads by name; its effects never run again.
    pub fn dispose(self) {
        dispose_owner(self.id, false);
    }
}

/// Registers `f` to run when the current owner is disposed, or before
/// the memo or effect that is running re-runs.
///
/// # Panics
/// With no current owner the cleanup could never run; that is a bug in
/// the caller, so it is refused loudly.
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    let registered = with_runtime(|runtime| {
        let owner = runtime.current_owner?;
        runtime.owner_mut(owner)?.cleanups.push(Box::new(f));
        Some(())
    });
    assert!(
        registered.is_some(),
        "on_cleanup called with no current owner: wrap the code in Owner::with"
    );
}

/// What disposal takes out of the runtime, to be dropped and run after
/// the runtime borrow is released (both may run user code).
#[derive(Default)]
pub(crate) struct Released {
    values: Vec<ValueCell>,
    computes: Vec<Compute>,
    cleanups: Vec<Cleanup>,
}

impl Released {
    fn finish(self) {
        drop(self.computes);
        drop(self.values);
        for cleanup in self.cleanups {
            cleanup();
        }
    }
}

/// Disposes an owner's children, nodes and cleanups, keeping the owner
/// itself for its next run.
pub(crate) fn dispose_contents(id: OwnerId) {
    dispose_owner(id, true);
}

fn dispose_owner(id: OwnerId, keep: bool) {
    let mut released = Released::default();
    with_runtime(|runtime| collect_owner(runtime, id, keep, &mut released));
    released.finish();
}

fn collect_owner(runtime: &mut Runtime, id: OwnerId, keep: bool, released: &mut Released) {
    let Some(owner) = runtime.owner_mut(id) else {
        return;
    };
    let children = std::mem::take(&mut owner.children);
    let nodes = std::mem::take(&mut owner.nodes);
    released
        .cleanups
        .extend(std::mem::take(&mut owner.cleanups));
    for child in children.into_iter().rev() {
        collect_owner(runtime, child, false, released);
    }
    for node in nodes.into_iter().rev() {
        collect_node(runtime, node, released);
    }
    if !keep {
        unlink_owner(runtime, id);
    }
}

fn unlink_owner(runtime: &mut Runtime, id: OwnerId) {
    let parent = runtime
        .owners
        .remove(id.index, id.generation)
        .and_then(|owner| owner.parent);
    if let Some(parent) = parent.and_then(|parent| runtime.owner_mut(parent)) {
        parent.children.retain(|child| *child != id);
    }
    if runtime.current_owner == Some(id) {
        runtime.current_owner = parent;
    }
}

/// Disposes one node explicitly (`signal.dispose()` and friends).
pub(crate) fn dispose_node(id: NodeId) {
    let mut released = Released::default();
    with_runtime(|runtime| {
        let owner = runtime.node(id).and_then(|node| node.owner);
        if let Some(owner) = owner.and_then(|owner| runtime.owner_mut(owner)) {
            owner.nodes.retain(|node| *node != id);
        }
        collect_node(runtime, id, &mut released);
    });
    released.finish();
}

fn collect_node(runtime: &mut Runtime, id: NodeId, released: &mut Released) {
    let Some(node) = runtime.nodes.remove(id.index, id.generation) else {
        return;
    };
    for source in &node.sources {
        if let Some(source) = runtime.node_mut(*source) {
            source.observers.retain(|observer| *observer != id);
        }
    }
    for observer in &node.observers {
        if let Some(observer) = runtime.node_mut(*observer) {
            observer.sources.retain(|source| *source != id);
        }
    }
    if let Some(scope) = node.scope {
        collect_owner(runtime, scope, false, released);
    }
    released.values.extend(node.value);
    released.computes.extend(node.compute);
}
