//! The processes of one browser, found by its profile: Chromium passes
//! `--user-data-dir=<profile>` to every process it starts (the browser,
//! zygotes, renderers, the GPU and utility processes; 8 of 8 on
//! 2026-11-22). The reused browser runs in the test's own process group,
//! so nextest's kill reaches it; its own deadline and cleanup therefore
//! kill these processes by pid, never a group, which is the test's.

use std::path::Path;
use std::time::{Duration, Instant};

/// How long killed processes may take to end. Measured 2026-11-14, 60
/// runs 12 at a time at load 76 to 114: 59 had ended at the first read
/// (under 0.1 ms), the slowest after 20 ms (`chromium_tests.rs`'s
/// `KILLED_ENDS_WITHIN`). 5 s is about 250 times that.
pub(super) const KILLED_END_WITHIN: Duration = Duration::from_secs(5);

/// The pids whose command line holds `--user-data-dir=<profile>` as one
/// whole argument. A process that ends while it is read, or one of
/// another user's that cannot be read, is not counted: neither can be
/// one of this browser's.
#[must_use]
pub(super) fn holding(profile: &Path) -> Vec<u32> {
    let wanted = format!("--user-data-dir={}", profile.display());
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .is_ok_and(|line| line.split(|&b| b == 0).any(|arg| arg == wanted.as_bytes()))
        })
        .collect()
}

/// SIGKILL to every process holding `profile`, again for any it starts
/// meanwhile, until every one has ended. A process started while the
/// first kill was sent still wrote to the profile, and removing it failed
/// with "Directory not empty", once under a full test run (2026-11-23).
///
/// # Errors
/// `kill` cannot run, or a process still runs [`KILLED_END_WITHIN`]
/// later, naming the pids.
pub(super) fn kill_holding(profile: &Path) -> Result<(), String> {
    let mut killed: Vec<u32> = Vec::new();
    let started = Instant::now();
    loop {
        let fresh: Vec<u32> = holding(profile)
            .into_iter()
            .filter(|pid| !killed.contains(pid))
            .collect();
        kill(&fresh)?;
        killed.extend(fresh);
        let left: Vec<u32> = killed.iter().copied().filter(|&pid| running(pid)).collect();
        if left.is_empty() && holding(profile).is_empty() {
            return Ok(());
        }
        if started.elapsed() >= KILLED_END_WITHIN {
            return Err(format!(
                "processes {left:?} of the browser on {} were still running {KILLED_END_WITHIN:?} \
                 after SIGKILL. Fix: kill them by hand. Reproduce: pgrep -af -- '--user-data-dir={0}'",
                profile.display()
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// SIGKILL to `pids`. Its exit status is not read: a pid that ended since
/// it was found makes `kill` exit 1, and what counts is whether any runs.
fn kill(pids: &[u32]) -> Result<(), String> {
    if pids.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = pids.iter().map(u32::to_string).collect();
    std::process::Command::new("kill")
        .args(["-s", "KILL", "--"])
        .args(&names)
        .output()
        .map(drop)
        .map_err(|e| format!("kill: {e}"))
}

/// Whether any thread of `pid` still runs. Its command line empties as
/// it exits, before its other threads have ended: profile files were
/// still written then, and removing the profile failed with "Directory
/// not empty" (2026-11-22). Gone is no `/proc/<pid>`, or a zombie with
/// no thread but its own.
fn running(pid: u32) -> bool {
    let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    let zombie = stat
        .rfind(')')
        .and_then(|end| stat[end + 1..].trim_start().chars().next())
        == Some('Z');
    let threads = std::fs::read_dir(format!("/proc/{pid}/task")).map_or(0, Iterator::count);
    !(zombie && threads <= 1)
}

#[cfg(test)]
#[path = "chromium_pids_tests.rs"]
mod tests;
