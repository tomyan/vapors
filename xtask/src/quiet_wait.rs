//! Whose quiet hold a run may go on under: the waits themselves are in
//! `share.rs` (before every share of the gate lock: `ci`, `test` and a
//! gate's shared steps) and `gate_hold.rs` (a gate's exclusive steps,
//! which take the lock).

use crate::quiet_lock::{Form, Holder};

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

/// Whether a gate takes a hold of its own given the `current` holder:
/// not when it is the gate's own (its parent `quiet`, or a `quiet hold`
/// from its worktree), whose hold already keeps others out.
pub fn gate_takes_a_hold(current: Option<&Holder>, parent: Option<u32>, here: &str) -> bool {
    !current.is_some_and(|holder| is_mine(holder, parent, here))
}

/// This process's working directory, as holders name it.
pub fn worktree() -> String {
    std::env::current_dir().map_or_else(|_| "?".to_owned(), |d| d.display().to_string())
}

#[cfg(test)]
#[path = "quiet_wait_tests.rs"]
mod tests;
