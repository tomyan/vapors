//! `cargo xtask ci` as the command runs it: sharing the gate lock, the
//! load wait, and sccache's hit rate over the run.

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
    if let Err(why) = load_wait::before("ci") {
        return fail("ci", &why);
    }
    let worktree =
        std::env::current_dir().map_or_else(|_| "?".to_owned(), |d| d.display().to_string());
    let shared = gate_lock::dir()
        .and_then(|dir| gate_lock_shared::acquire_shared(&dir, &worktree, gate_lock::WAIT));
    match shared {
        Ok(_shared) => with_hit_rate(steps, reporter),
        Err(why) => fail("ci", &why),
    }
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
