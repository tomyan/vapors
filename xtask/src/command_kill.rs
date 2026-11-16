//! Ending a step's process group: SIGTERM first so its traps run, then
//! SIGKILL to what is left.

use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// SIGTERM to the step's process group, then, once the leader has
/// ended or [`TERM_GRACE`] has passed, SIGKILL to what is left, then
/// reap the leader. The TERM lets a script's traps run: the page
/// contract removes its work directory on exit, and killed outright it
/// left about 80 files in the gate's TMPDIR. Signals go by `kill -s SIG
/// -- -<pid>`, procps's documented form for a group. With `kill -KILL
/// -<pid>` the group survived on the runner: vapors run 40006425576,
/// where the gate's deadline test took 21 s, and likewise vapors #30.
/// The KILL's own failure is returned to be reported. Declared limit:
/// the leader is reaped in the grace loop, before the KILL to its
/// group, so if every member has gone the group id could in principle
/// be reused in between (microseconds, against a 4M pid space).
pub(crate) fn kill_group(child: &mut Child) -> Result<(), String> {
    // The TERM's own failure is not returned: the KILL follows after the
    // grace either way, and its failure is.
    let _ = signal_group(child.id(), "TERM");
    let asked = Instant::now();
    while child.try_wait().ok().flatten().is_none() && asked.elapsed() < TERM_GRACE {
        std::thread::sleep(Duration::from_millis(20));
    }
    let killed = kill_group_id(child.id());
    let _ = child.kill();
    let _ = child.wait();
    killed
}

/// How long a step's group has to end after SIGTERM before SIGKILL.
/// Measured 2026-11-13 at load 37 to 55 with this file's test step (a
/// bash with an exit trap that removes its work directory, and a child
/// in the background): it ended 27 to 56 ms after the TERM in six runs.
/// The page contract's trap does the same kind of work (stop its
/// server, remove about 80 files). 5 s is about 90 times the worst.
pub const TERM_GRACE: Duration = Duration::from_secs(5);

/// Kill process group `group`; a group already gone counts as killed.
///
/// # Errors
///
/// `kill` could not run, or refused for another reason.
pub(crate) fn kill_group_id(group: u32) -> Result<(), String> {
    signal_group(group, "KILL")
}

/// Send `signal` to process group `group`; a group already gone counts
/// as signalled.
fn signal_group(group: u32, signal: &str) -> Result<(), String> {
    let group = format!("-{group}");
    let sent = Command::new("kill")
        .args(["-s", signal, "--", &group])
        .output();
    match sent {
        Ok(out) if out.status.success() => Ok(()),
        // ESRCH: every process of the group had exited by the signal.
        Ok(out) if String::from_utf8_lossy(&out.stderr).contains("No such process") => Ok(()),
        Ok(out) => Err(format!(
            "kill -s {signal} -- {group}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("kill: {e}")),
    }
}

/// SIGTERM to each of `groups`, then wait until every one has gone or
/// [`TERM_GRACE`] has passed, so their traps run before the caller's
/// SIGKILL. For the queue's kill of a whole gate (`queue_gate_child.rs`).
pub(crate) fn ask_groups_to_end(groups: &[u32]) {
    for group in groups {
        // Not returned: the caller's KILL follows, and its failure is.
        let _ = signal_group(*group, "TERM");
    }
    let asked = Instant::now();
    while groups.iter().any(|group| group_exists(*group)) && asked.elapsed() < TERM_GRACE {
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Whether process group `group` has a member: `kill -s 0` succeeds.
fn group_exists(group: u32) -> bool {
    Command::new("kill")
        .args(["-s", "0", "--", &format!("-{group}")])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
#[path = "command_kill_tests.rs"]
mod tests;
