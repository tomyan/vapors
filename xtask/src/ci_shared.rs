//! `cargo xtask ci` as the command runs it: the quiet lock's wait,
//! sharing the gate lock, the load wait, and sccache's hit rate over the run.

use std::process::ExitCode;

use crate::{
    ci, fail, gate_lock, gate_lock_shared, load_wait, priority, report::Reporter, sccache, steps,
};

/// `cargo xtask ci`, sharing the gate lock so it never runs beside a gate
/// (`gate_lock_shared.rs`). Relaxed inside a gate's step (`XTASK_GATE=1`):
/// the gate already holds the lock, and taking a share would wait on the
/// gate itself. It would retire if no gate step could run `ci`.
pub fn run(steps: &[steps::Step], reporter: &Reporter) -> ExitCode {
    let gate_env = std::env::var(priority::GATE_ENV).ok();
    if priority::in_gate(false, gate_env.as_deref()) {
        return with_hit_rate(steps, reporter);
    }
    match quiet_then_share() {
        Ok(_shared) => with_hit_rate(steps, reporter),
        Err(why) => fail("ci", &why),
    }
}

/// How many times a ci run waits for the quiet lock again when it was
/// taken between its wait and its share. RELAXED: chosen, not measured;
/// no log yet shows a hold landing in a ci run's load wait. Each round
/// is a whole quiet window (up to 60 min), so three bounds a ci run's
/// wait at about three hours, and a fourth fails naming the fix.
/// Retired by a count taken from the logs of the re-check's line.
const QUIET_ROUNDS: u32 = 3;

/// Wait for the quiet lock and the load, then take a share of the gate
/// lock; if the quiet lock was taken meanwhile, let the share go so the
/// holder's drain can finish, and wait again.
fn quiet_then_share() -> Result<gate_lock_shared::Shared, String> {
    let dir = gate_lock::dir()?;
    let worktree = crate::quiet_wait::worktree();
    for _ in 0..QUIET_ROUNDS {
        crate::quiet_wait::before("ci")?;
        load_wait::before("ci")?;
        let shared = gate_lock_shared::acquire_shared(&dir, &worktree, gate_lock::WAIT)?;
        // RELAXED: no test drives this re-check (it needs a quiet lock
        // taken between two real waits); `taken_since` and the waits it
        // repeats are tested. Retired by a test that takes the lock
        // during a ci run's load wait.
        if !crate::quiet_wait::taken_since(&dir)? {
            return Ok(shared);
        }
        println!("ci: the quiet lock was taken while this run waited; letting its share go");
    }
    Err(format!(
        "the quiet lock was taken {QUIET_ROUNDS} times between this run's wait and its share; \
         fix: rerun `cargo xtask ci` once the quiet holders are done"
    ))
}

/// `cargo xtask ci`, then sccache's hit rate over the run; sccache must
/// answer before and after.
fn with_hit_rate(steps: &[steps::Step], reporter: &Reporter) -> ExitCode {
    let before = match sccache::stats() {
        Ok(stats) => stats,
        Err(why) => return fail("sccache", &why),
    };
    let code = ci::run(steps, reporter);
    match sccache::stats() {
        Ok(after) => match sccache::hit_rate(before, after) {
            Ok(line) => println!("{line}"),
            Err(why) => return fail("sccache", &why),
        },
        Err(why) => return fail("sccache", &why),
    }
    code
}
