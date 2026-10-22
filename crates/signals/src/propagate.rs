//! Marking (push) and updating (pull): the three-state algorithm that
//! keeps memos glitch-free and runs each computation at most once per
//! change.

use crate::guard::Restore;
use crate::node::{NodeId, OwnerId, State};
use crate::owner;
use crate::runtime::{Runtime, with_runtime};
use crate::track::clear_sources;

/// Marks every observer of `source` dirty, and their observers
/// "check", scheduling the effects reached.
pub(crate) fn notify_observers(runtime: &mut Runtime, source: NodeId) {
    let observers = match runtime.node(source) {
        Some(node) => node.observers.clone(),
        None => return,
    };
    for observer in observers {
        mark_dirty(runtime, observer);
    }
}

fn mark_dirty(runtime: &mut Runtime, id: NodeId) {
    let Some(node) = runtime.node_mut(id) else {
        return;
    };
    match node.state {
        State::Dirty => {}
        State::Check => node.state = State::Dirty,
        State::Clean => {
            node.state = State::Dirty;
            let observers = node.observers.clone();
            schedule_if_effect(runtime, id);
            for observer in observers {
                mark_check(runtime, observer);
            }
        }
    }
}

fn mark_check(runtime: &mut Runtime, id: NodeId) {
    let Some(node) = runtime.node_mut(id) else {
        return;
    };
    if node.state != State::Clean {
        return;
    }
    node.state = State::Check;
    let observers = node.observers.clone();
    schedule_if_effect(runtime, id);
    for observer in observers {
        mark_check(runtime, observer);
    }
}

fn schedule_if_effect(runtime: &mut Runtime, id: NodeId) {
    let Some(node) = runtime.node_mut(id) else {
        return;
    };
    if node.info.kind == crate::Kind::Effect && !node.scheduled {
        node.scheduled = true;
        runtime.pending.push(id);
    }
}

/// Brings `id` up to date: a "check" node asks its sources first and
/// recomputes only if one of them changed; a dirty node recomputes.
pub(crate) fn update_if_necessary(id: NodeId) {
    let Some(state) = with_runtime(|runtime| runtime.node(id).map(|node| node.state)) else {
        return;
    };
    if state == State::Check {
        check_sources(id);
    }
    let state = with_runtime(|runtime| runtime.node(id).map(|node| node.state));
    if state == Some(State::Dirty) {
        recompute(id);
    } else {
        with_runtime(|runtime| {
            if let Some(node) = runtime.node_mut(id) {
                node.state = State::Clean;
            }
        });
    }
}

fn check_sources(id: NodeId) {
    let sources = with_runtime(|runtime| runtime.node(id).map(|node| node.sources.clone()));
    for source in sources.unwrap_or_default() {
        update_if_necessary(source);
        let dirty = with_runtime(|runtime| runtime.node(id).map(|node| node.state));
        if dirty == Some(State::Dirty) {
            return;
        }
    }
}

/// Runs the computation of a memo or effect: its previous run's
/// subscriptions and nested nodes are dropped first, and if a memo's
/// value changed its observers are marked dirty.
pub(crate) fn recompute(id: NodeId) {
    let Some((compute, scope)) = with_runtime(|runtime| {
        clear_sources(runtime, id);
        let node = runtime.node_mut(id)?;
        node.state = State::Clean;
        Some((node.compute.clone()?, node.scope))
    }) else {
        return;
    };
    if let Some(scope) = scope {
        owner::dispose_contents(scope);
    }
    let changed = run_computing(id, scope, || (compute.borrow_mut())());
    if changed {
        with_runtime(|runtime| notify_observers(runtime, id));
    }
}

/// Runs `compute` as `id`: tracked, on the computing stack, with its
/// scope as the owner. All three are restored on return or unwind.
fn run_computing(id: NodeId, scope: Option<OwnerId>, compute: impl FnOnce() -> bool) -> bool {
    let previous_owner = with_runtime(|runtime| {
        runtime.computing.push(id);
        runtime.observers.push(Some(id));
        std::mem::replace(&mut runtime.current_owner, scope)
    });
    let _restore = Restore::new(move || {
        with_runtime(|runtime| {
            runtime.computing.pop();
            runtime.observers.pop();
            runtime.current_owner = previous_owner;
            if std::thread::panicking() {
                settle_after_panic(runtime, id);
            }
        });
    });
    compute()
}

/// After a panic: a memo is left dirty, so the next read computes it
/// again; an effect is left clean with the subscriptions it made before
/// panicking, so the next change to one of them runs it again.
fn settle_after_panic(runtime: &mut Runtime, id: NodeId) {
    if let Some(node) = runtime.node_mut(id) {
        node.state = match node.info.kind {
            crate::Kind::Effect => State::Clean,
            _ => State::Dirty,
        };
    }
}
