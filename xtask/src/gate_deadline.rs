//! The longest a gate may take, from the waits of its holds
//! (`gate_hold.rs`), its jobs' `timeout-minutes`, and the deadlines of
//! its own commands outside its steps (Tom, 2026-11-23): the deadline of
//! a second gate waiting in the same worktree (`gate_worktree_lock.rs`)
//! and of a gate the queue runs (`queue_gate_child.rs`).
//!
//! Declared limits: `[load] deadline_minutes` set above the default
//! would lengthen each hold. The gate's reads in its own process (the
//! settings, the workflow and its plan, the twin's files, the run
//! TMPDIR's sweep, /proc) have no deadline of their own, since there is
//! no child to kill; each is a read of local files. Either can make a gate
//! outlive [`LONGEST`], which then kills or refuses it, loudly.

use std::time::Duration;

/// The longest one hold waits: the quiet lock (`quiet_lock::WAIT`, 61
/// min), then the turn, a slot and the gate lock together
/// (`gate_lock::WAIT`, 40 min), then the load wait, which goes on at its
/// default deadline (`load_wait::DEFAULT_DEADLINE`, 10 min): 111 min.
/// A gate's share tries rounds until the first two together, 101 min
/// (`share.rs`); each quiet and lock wait is cut to what is left when it
/// starts, and only a load wait begun just before that deadline can pass
/// it, by at most its own 10 min, so the share too keeps within 111.
pub const HOLD_WAIT: Duration = crate::quiet_lock::WAIT
    .saturating_add(crate::gate_lock::WAIT)
    .saturating_add(crate::load_wait::DEFAULT_DEADLINE);

/// How many holds a gate takes over .github/workflows/ci.yml: shared up
/// to the trunk build, exclusive after. A test holds it to the tables.
pub const HOLDS: u32 = 2;

/// ci.yml's jobs' `timeout-minutes` together, 9 + 11 + 13 + 9: the
/// longest its steps may run. A test holds it to the workflow.
pub const JOBS: Duration = Duration::from_mins(42);

/// The checkouts whose state the gate reads: this one and each
/// `[gate.pinned]` sibling, of which vapors has none. A test holds it to
/// the settings.
pub const CHECKOUTS: u32 = 1;

/// The gate's own commands outside its steps, each at its deadline: two
/// `stat`s for the disk floor (`gate_floor::STAT_DEADLINE`, 10 s), the
/// sandbox check (`gate_sandbox::CHECK_DEADLINE`, 10 s), and two git
/// calls per checkout for its state before the steps and again after
/// (`gate_state::GIT_DEADLINE`, 30 s): 20 + 10 + 120 s = 150 s.
pub const OWN_COMMANDS: Duration = crate::gate::floor::STAT_DEADLINE
    .saturating_mul(2)
    .saturating_add(crate::gate_run::SANDBOX_CHECK_DEADLINE)
    .saturating_add(crate::gate_state::GIT_DEADLINE.saturating_mul(4 * CHECKOUTS));

/// The longest a gate runs: its jobs, each hold's longest wait, and its
/// own commands, 42 min + 2 x 111 min + 150 s = 266.5 min.
pub const LONGEST: Duration = JOBS
    .saturating_add(HOLD_WAIT.saturating_mul(HOLDS))
    .saturating_add(OWN_COMMANDS);

#[cfg(test)]
#[path = "gate_deadline_tests.rs"]
pub(crate) mod tests;
