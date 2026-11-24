//! The processes of one browser, found by its profile: Chromium passes
//! `--user-data-dir=<profile>` to every process it starts (the browser,
//! zygotes, renderers, the GPU and utility processes). The browser's own
//! command line keeps its arguments NUL-separated; every process it starts
//! rewrites its title, so its command line is one string of space-joined
//! arguments ended by a single NUL (9 processes holding the profile on
//! 2026-11-23, 1 of them NUL-separated; the review of #615 checked with
//! `od -c`). The crashpad handler it starts holds no profile argument and
//! ends by itself after the browser. The reused browser runs in the test's own process group,
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
/// whole argument, in either form: NUL-separated, or space-joined in a
/// rewritten title, where it is preceded by the start or a space and
/// followed by a space or the end (so `<profile>2` is another profile). A
/// process that ends while it is read is absent.
///
/// # Errors
/// `/proc` cannot be listed, or a process's command line cannot be read
/// for a reason other than its having ended, naming it.
pub(super) fn holding(profile: &Path) -> Result<Vec<u32>, String> {
    holding_in(Path::new("/proc"), profile)
}

/// [`holding`], with `proc` as the process table.
fn holding_in(proc: &Path, profile: &Path) -> Result<Vec<u32>, String> {
    let wanted = format!("--user-data-dir={}", profile.display());
    let listing = |e: std::io::Error| format!("listing {}: {e}", proc.display());
    let mut found = Vec::new();
    for entry in std::fs::read_dir(proc).map_err(listing)? {
        let entry = entry.map_err(listing)?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        if command_line(&entry.path().join("cmdline"))?
            .is_some_and(|line| holds(&line, wanted.as_bytes()))
        {
            found.push(pid);
        }
    }
    Ok(found)
}

/// The command line at `path`, or none when its process has ended.
fn command_line(path: &Path) -> Result<Option<Vec<u8>>, String> {
    const ESRCH: i32 = 3;
    match std::fs::read(path) {
        Ok(line) => Ok(Some(line)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound || e.raw_os_error() == Some(ESRCH) => {
            Ok(None)
        }
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Whether `line` holds `wanted` as a whole argument, NUL-separated or
/// space-joined.
fn holds(line: &[u8], wanted: &[u8]) -> bool {
    line.split(|&b| b == 0).any(|arg| {
        arg.windows(wanted.len()).enumerate().any(|(at, window)| {
            let before = at.checked_sub(1).map(|i| arg[i]);
            let after = arg.get(at + wanted.len()).copied();
            window == wanted
                && matches!(before, None | Some(b' '))
                && matches!(after, None | Some(b' '))
        })
    })
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
        let fresh: Vec<u32> = holding(profile)?
            .into_iter()
            .filter(|pid| !killed.contains(pid))
            .collect();
        kill(&fresh)?;
        killed.extend(fresh);
        let left: Vec<u32> = killed.iter().copied().filter(|&pid| running(pid)).collect();
        if left.is_empty() && holding(profile)?.is_empty() {
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

/// Kill every process holding `profile` (Chromium and all it started),
/// then reap `child`.
pub(super) fn stop_holding(
    child: &mut std::process::Child,
    profile: &Path,
) -> Result<String, String> {
    let killed = kill_holding(profile);
    let _ = child.kill();
    let _ = child.wait();
    killed.map(|()| "killed with every process holding its profile".to_owned())
}

/// SIGKILL to `pids`. Its exit status is not read: a pid that ended since
/// it was found makes `kill` exit 1, and what counts is whether any runs.
///
/// RELAXED: a pid that ends and is reused by another process between
/// `holding` reading its command line and this kill would kill that
/// process. `pidfd_open` and `pidfd_send_signal` close the gap, but std
/// has no stable pidfd and the workspace forbids `unsafe` (and takes no
/// libc binding for one call). The gap is the few milliseconds between
/// two reads of `/proc` and pids cycle through 4194304 here
/// (`/proc/sys/kernel/pid_max`). Retired by a stable `std::os::linux::process::PidFd`
/// that can signal a process it did not spawn.
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

#[cfg(test)]
#[path = "chromium_group_kill_tests.rs"]
mod group_kill_tests;
