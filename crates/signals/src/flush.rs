//! Batching and the flush loop that runs scheduled effects.

use std::cell::RefCell;
use std::collections::VecDeque;

use crate::error::NodeInfo;
use crate::guard::Restore;
use crate::node::NodeId;
use crate::propagate::update_if_necessary;
use crate::runtime::with_runtime;

/// How many passes a flush may take before it is a runaway loop. The
/// ported `recursive_effect_runs_recursively` converges in two passes
/// and no component in this repository needs more than a handful; a
/// hundred bounds a bug to milliseconds instead of a hang.
const MAX_FLUSH_PASSES: u32 = 100;

/// Defers effects until `f` returns; batches nest and flush once at the
/// end of the outermost one. A memo reading several signals written in
/// the batch recomputes once.
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    with_runtime(|runtime| runtime.batch_depth += 1);
    let depth = Restore::new(|| with_runtime(|runtime| runtime.batch_depth -= 1));
    let result = f();
    drop(depth);
    if with_runtime(|runtime| runtime.batch_depth == 0) {
        flush();
    }
    result
}

/// Called after a signal write: runs effects now unless a batch or a
/// flush is in progress, in which case the write is picked up by them.
pub(crate) fn after_write() {
    let flush_now = with_runtime(|runtime| runtime.batch_depth == 0 && !runtime.flushing);
    if flush_now {
        flush();
    }
}

fn flush() {
    let already = with_runtime(|runtime| std::mem::replace(&mut runtime.flushing, true));
    if already {
        return;
    }
    let _flushing = Restore::new(|| with_runtime(|runtime| runtime.flushing = false));
    for pass in 1.. {
        let effects = take_pending();
        if effects.is_empty() {
            return;
        }
        if pass > MAX_FLUSH_PASSES {
            refuse_runaway(&effects);
        }
        run_pass(effects);
    }
}

/// Runs one pass in order. If an effect panics, the ones after it are
/// scheduled again, so the next flush runs them rather than losing them.
fn run_pass(effects: Vec<NodeId>) {
    let queue = RefCell::new(VecDeque::from(effects));
    let _unrun = Restore::new(|| reschedule(queue.borrow_mut().drain(..)));
    loop {
        let next = queue.borrow_mut().pop_front();
        let Some(effect) = next else {
            break;
        };
        update_if_necessary(effect);
    }
}

fn reschedule(effects: impl Iterator<Item = NodeId>) {
    with_runtime(|runtime| {
        for id in effects {
            if let Some(node) = runtime.node_mut(id) {
                node.scheduled = true;
                runtime.pending.push(id);
            }
        }
    });
}

/// The scheduled effects in creation order, with their flags cleared
/// so a write during this pass schedules them again for the next.
fn take_pending() -> Vec<NodeId> {
    with_runtime(|runtime| {
        let pending = std::mem::take(&mut runtime.pending);
        let mut ordered: Vec<(u64, NodeId)> = pending
            .into_iter()
            .filter_map(|id| {
                let node = runtime.node_mut(id)?;
                node.scheduled = false;
                Some((node.seq, id))
            })
            .collect();
        ordered.sort_unstable_by_key(|(seq, _)| *seq);
        ordered.into_iter().map(|(_, id)| id).collect()
    })
}

fn refuse_runaway(effects: &[NodeId]) -> ! {
    let names: Vec<NodeInfo> = with_runtime(|runtime| {
        effects
            .iter()
            .filter_map(|id| Some(runtime.node(*id)?.info))
            .collect()
    });
    let listed: Vec<String> = names.iter().map(ToString::to_string).collect();
    panic!(
        "effects re-dirtied themselves {MAX_FLUSH_PASSES} passes in a row: {}; an effect must not write a signal it reads unconditionally",
        listed.join(", ")
    );
}
