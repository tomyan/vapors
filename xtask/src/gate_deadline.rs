//! The longest a gate may take, from the waits of its holds
//! (`gate_hold.rs`) and its jobs' `timeout-minutes`: the deadline of a
//! second gate waiting in the same worktree (`gate_worktree_lock.rs`)
//! and of a gate the queue runs (`queue_gate_child.rs`).
//!
//! Declared limits: a share waits for the quiet lock again when one was
//! taken between its wait and its share (`share.rs`, at most three
//! rounds); only the first round is counted here. `[load]
//! deadline_minutes` set above the default would also lengthen each hold.
//! Either can make a gate outlive [`LONGEST`], which then kills or
//! refuses it, loudly.

use std::time::Duration;

/// The longest one hold waits: the quiet lock (`quiet_lock::WAIT`, 61
/// min), then the turn, a slot and the gate lock together
/// (`gate_lock::WAIT`, 40 min), then the load wait, which goes on at its
/// default deadline (`load_wait::DEFAULT_DEADLINE`, 10 min): 111 min.
pub const HOLD_WAIT: Duration = crate::quiet_lock::WAIT
    .saturating_add(crate::gate_lock::WAIT)
    .saturating_add(crate::load_wait::DEFAULT_DEADLINE);

/// How many holds a gate takes over .github/workflows/ci.yml: shared up
/// to the trunk build, exclusive after. A test holds it to the tables.
pub const HOLDS: u32 = 2;

/// ci.yml's jobs' `timeout-minutes` together, 9 + 11 + 13 + 9: the
/// longest its steps may run. A test holds it to the workflow.
pub const JOBS: Duration = Duration::from_mins(42);

/// The longest a gate runs: its jobs, and each hold's longest wait,
/// 42 + 2 x 111 = 264 min.
pub const LONGEST: Duration = JOBS.saturating_add(HOLD_WAIT.saturating_mul(HOLDS));

#[cfg(test)]
#[path = "gate_deadline_tests.rs"]
pub(crate) mod tests;
