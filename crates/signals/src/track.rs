//! Dependency tracking: which observer is running, and subscribing the
//! sources it reads.

use crate::node::NodeId;
use crate::runtime::{Runtime, with_runtime};

/// Runs `f` without subscribing the running memo or effect to anything
/// it reads.
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    with_runtime(|runtime| runtime.observers.push(None));
    let result = f();
    with_runtime(|runtime| runtime.observers.pop());
    result
}

/// Records that the running observer, if any, read `source`.
pub(crate) fn track_read(runtime: &mut Runtime, source: NodeId) {
    let Some(Some(observer)) = runtime.observers.last().copied() else {
        return;
    };
    subscribe(runtime, observer, source);
}

fn subscribe(runtime: &mut Runtime, observer: NodeId, source: NodeId) {
    if let Some(node) = runtime.node_mut(source)
        && !node.observers.contains(&observer)
    {
        node.observers.push(observer);
    }
    if let Some(node) = runtime.node_mut(observer)
        && !node.sources.contains(&source)
    {
        node.sources.push(source);
    }
}

/// Drops every subscription `observer` holds, before it re-runs.
pub(crate) fn clear_sources(runtime: &mut Runtime, observer: NodeId) {
    let Some(node) = runtime.node_mut(observer) else {
        return;
    };
    let sources = std::mem::take(&mut node.sources);
    for source in sources {
        if let Some(node) = runtime.node_mut(source) {
            node.observers.retain(|id| *id != observer);
        }
    }
}
