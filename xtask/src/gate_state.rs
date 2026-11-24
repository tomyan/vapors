//! The checkout's state, read before the gate's steps and again after:
//! HEAD, whether the tree has changes, and each pinned sibling's. Every
//! git call is within [`GIT_DEADLINE`].

use std::process::Command;
use std::time::Duration;

/// How a state with changes in its tree is marked.
pub const DIRTY: &str = " (dirty)";

/// A git call's deadline. Measured 2026-11-23 in this worktree, 20 runs
/// each at load 1.2: `git status --porcelain` took 8.4 ms (median) to
/// 9.3 ms, `git rev-parse HEAD` 2.1 to 2.4 ms. 30 s is over 3,000 times
/// the longest, for a machine far busier or a cold disk cache.
pub const GIT_DEADLINE: Duration = Duration::from_secs(30);

/// HEAD, whether the tree has changes, and each pinned sibling's.
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    /// HEAD's SHA, ` (dirty)` when the tree has changes.
    pub head: String,
    /// Each sibling's HEAD, which equals its pin and is clean.
    pub siblings: Vec<String>,
}

pub fn state(siblings: &[(String, String)]) -> Result<State, String> {
    let head = checkout(".")?;
    let siblings = siblings
        .iter()
        .map(|(dir, pin_file)| pinned(dir, pin_file))
        .collect::<Result<_, _>>()?;
    Ok(State { head, siblings })
}

/// A sibling checkout, which must be clean and at its pin.
fn pinned(dir: &str, pin_file: &str) -> Result<String, String> {
    let read = || std::fs::read_to_string(pin_file).map_err(|e| format!("{pin_file}: {e}"));
    let part = format!("the pin file {pin_file}");
    let text = crate::gate_watchdog::watched(part, crate::gate_watchdog::IN_PROCESS, read)?;
    let pin = text
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap_or_default()
        .trim()
        .to_owned();
    let head = checkout(dir)?;
    if head != pin {
        return Err(format!(
            "{dir} is at {head}, not {pin} from {pin_file}, or has changes; fix: git -C {dir} checkout --detach {pin} on a clean tree"
        ));
    }
    Ok(head)
}

/// `dir`'s HEAD SHA, ` (dirty)` when it has changes; git must exit 0.
fn checkout(dir: &str) -> Result<String, String> {
    let sha = git(dir, &["rev-parse", "HEAD"])?;
    let changes = git(dir, &["status", "--porcelain"])?;
    Ok(if changes.is_empty() {
        sha
    } else {
        format!("{sha}{DIRTY}")
    })
}

/// git in `dir`, its stdout trimmed; hook variables stripped (CLAUDE.md);
/// within [`GIT_DEADLINE`].
///
/// # Errors
///
/// git not starting, passing its deadline (naming the call), or not
/// exiting 0.
pub fn git(dir: &str, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    let call = format!("git -C {dir} {}", args.join(" "));
    output_of(command, &call, GIT_DEADLINE)
}

/// `command`'s stdout, trimmed, within `deadline`; it must exit 0.
///
/// # Errors
///
/// It did not start, passed `deadline` (naming `call` and the part of
/// the gate it is), or did not exit 0.
pub fn output_of(command: Command, call: &str, deadline: Duration) -> Result<String, String> {
    let ended = crate::api_timed::within(command, deadline).map_err(|why| {
        format!(
            "the gate's checkout state, `{call}`: {why}; \
             fix: find what holds the repository (`ps`, a lock under .git), then rerun `cargo xtask gate`"
        )
    })?;
    if !ended.status.success() {
        return Err(format!(
            "{call}: {}",
            String::from_utf8_lossy(&ended.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ended.stdout).trim().to_owned())
}

#[cfg(test)]
#[path = "gate_state_tests.rs"]
mod tests;
