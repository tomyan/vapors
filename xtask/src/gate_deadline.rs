//! The longest a gate may take, from the waits of its holds
//! (`gate_hold.rs`), its jobs' `timeout-minutes`, and the deadlines of
//! its own commands outside its steps (Tom, 2026-11-23): the deadline of
//! a second gate waiting in the same worktree (`gate_worktree_lock.rs`)
//! and of a gate the queue runs (`queue_gate_child.rs`).
//!
//! Declared limit: `[load] deadline_minutes` set above the default
//! would lengthen each hold past [`HOLD_WATCHED`]; the gate's watchdog
//! then fails it, naming the hold (`gate_watchdog.rs`).

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

/// The watchdog's deadline on one hold (`gate_run.rs`): its longest
/// wait, and a minute for its own work beside the waits: the quiet
/// lock's claim and its renewal each wait at most 10 s for other takers
/// (`quiet_file::CLAIM_WAIT`), and its reads of lock files and /proc
/// take milliseconds (the process tests' holds, 12 to 36 ms each).
pub const HOLD_WATCHED: Duration = HOLD_WAIT.saturating_add(Duration::from_mins(1));

/// The steps of .github/workflows/ci.yml the gate runs. A test holds it
/// to the plan.
pub const STEPS: u32 = 31;

/// The gate's parts that do only local reads and writes, each watched
/// for `gate_watchdog::IN_PROCESS`: the settings, the twin check, the
/// workflow with its plan and holds, the `GITHUB_OUTPUT` file, the disk
/// floor's target directory, the run TMPDIR's sweep and its leftover
/// check (7); each sibling's pin file before and after; and for each
/// step its load readings before and after and its start's own reads
/// (3 per step): 7 + 3 x 31 = 100.
pub const IN_PROCESS_PARTS: u32 = 7 + 2 * (CHECKOUTS - 1) + 3 * STEPS;

/// The longest a gate runs, each part at its deadline: its jobs, each
/// hold as watched, its own commands, and its in-process parts,
/// 42 min + 2 x 112 min + 150 s + 100 x 5 s = 276 min 50 s.
pub const LONGEST: Duration = JOBS
    .saturating_add(HOLD_WATCHED.saturating_mul(HOLDS))
    .saturating_add(OWN_COMMANDS)
    .saturating_add(crate::gate_watchdog::IN_PROCESS.saturating_mul(IN_PROCESS_PARTS));

#[cfg(test)]
#[path = "gate_deadline_tests.rs"]
pub(crate) mod tests;
