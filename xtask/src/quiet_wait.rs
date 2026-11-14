//! Waiting on the quiet lock (`quiet_lock.rs`): before every `ci`,
//! `test` and `gate` run, again once a ci run has its share of the gate
//! lock, and the gate's own hold.
//!
//! Declared limit: `cargo xtask test` takes no share of the gate lock,
//! so `quiet`'s drain does not wait for test runs already past their
//! wait; they finish beside the quiet window. Retired if `test` takes a
//! share as ci does.

use crate::quiet_lock::{self, Form, Holder, LONGEST, Quiet, WAIT};

/// Set on the command `quiet` runs to the holder's pid, so the xtask
/// runs it starts do not wait on their own holder.
pub const HOLDER_ENV: &str = "XTASK_QUIET_HOLDER";

/// The pid in [`HOLDER_ENV`], if set.
///
/// # Errors
///
/// It is set to something other than a pid, naming it.
pub fn holder_from_env(value: Option<&str>) -> Result<Option<u32>, String> {
    let Some(text) = value else {
        return Ok(None);
    };
    text.parse().map(Some).map_err(|_| {
        format!("{HOLDER_ENV} is `{text}`, not a pid; fix: unset it, it is set only by `cargo xtask quiet`")
    })
}

/// Whether `holder` is this run's own: the `quiet` that started it, or a
/// hold taken from this worktree.
pub fn is_mine(holder: &Holder, parent: Option<u32>, here: &str) -> bool {
    Some(holder.owner.pid) == parent || (holder.form == Form::Hold && holder.owner.worktree == here)
}

/// Before `who` (`ci`, `test`) starts: wait while someone else holds the
/// lock. Not in a gate's step, whose gate holds it.
///
/// # Errors
///
/// As [`quiet_lock::wait_clear`], or [`holder_from_env`].
pub fn before(who: &str) -> Result<(), String> {
    let gate_env = std::env::var(crate::priority::GATE_ENV).ok();
    if crate::priority::in_gate(false, gate_env.as_deref()) {
        return Ok(());
    }
    let parent = holder_from_env(std::env::var(HOLDER_ENV).ok().as_deref())?;
    let (dir, here) = (crate::gate_lock::dir()?, worktree());
    let mine = |holder: &Holder| is_mine(holder, parent, &here);
    quiet_lock::wait_clear(&dir, (who, &mine), WAIT, &mut |line| println!("{line}"))
}

/// After a ci run has its share: whether someone else took the lock
/// since its wait, so it must let the share go and wait again.
///
/// # Errors
///
/// As [`quiet_lock::current`], or [`holder_from_env`].
pub fn taken_since(dir: &std::path::Path) -> Result<bool, String> {
    let parent = holder_from_env(std::env::var(HOLDER_ENV).ok().as_deref())?;
    let here = worktree();
    Ok(quiet_lock::current(dir)?.is_some_and(|holder| !is_mine(&holder, parent, &here)))
}

/// Whether a gate takes a hold of its own given the `current` holder:
/// not when it is the gate's own (its parent `quiet`, or a `quiet hold`
/// from its worktree), whose hold already keeps others out.
pub fn gate_takes_a_hold(current: Option<&Holder>, parent: Option<u32>, here: &str) -> bool {
    !current.is_some_and(|holder| is_mine(holder, parent, here))
}

/// The gate's hold, for [`LONGEST`]; none when the holder now is its own
/// ([`gate_takes_a_hold`]).
///
/// # Errors
///
/// As [`quiet_lock::take`], or [`holder_from_env`].
pub fn for_gate(worktree: &str) -> Result<Option<Quiet>, String> {
    let dir = crate::gate_lock::dir()?;
    let parent = holder_from_env(std::env::var(HOLDER_ENV).ok().as_deref())?;
    if !gate_takes_a_hold(quiet_lock::current(&dir)?.as_ref(), parent, worktree) {
        return Ok(None);
    }
    let holder = Holder::me(worktree, LONGEST, Form::Gate, "gate");
    quiet_lock::take(&dir, holder, WAIT).map(Some)
}

/// This process's working directory, as holders name it.
pub fn worktree() -> String {
    std::env::current_dir().map_or_else(|_| "?".to_owned(), |d| d.display().to_string())
}

#[cfg(test)]
#[path = "quiet_wait_tests.rs"]
mod tests;
