//! One watchdog for the gate's work in its own process (Tom,
//! 2026-11-24): its reads of the settings, the workflow and the twin's
//! files, the run TMPDIR's sweep, /proc, and the start of each step. A
//! part is watched by a thread of its own; when the part ends in time
//! the thread ends with it. A thread cannot stop a read that is stuck,
//! so at the deadline it says which part did not finish, as the gate's
//! failure, and ends the gate's process with exit code 1. The kernel
//! frees the gate's flocks then, and the quiet lock's holder file reads
//! as ended once its pid is gone (`quiet_holder.rs`). A step's own
//! process group is not killed by that exit; the step's part is watched
//! to its own deadline and a little more, so by then the step was killed
//! at its deadline (`command.rs`), unless the kill itself hung.

use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::time::Duration;

/// The deadline of a part that does only local reads and writes.
/// Measured 2026-11-23 and 24, 20 runs each at load 1.2 to 6.3, the
/// most each took: the settings 0.21 ms; the workflow and its plan
/// 1.5 ms; the twin check of xtask against itself 7.6 ms; the hold table
/// 0.04 ms; a /proc/loadavg reading 0.01 ms; the TMPDIR sweep of /tmp
/// (588 entries) 0.30 ms; the leftover check of 1,275 entries (a ci
/// run's measured peak, `gate_floor.rs`) 15.6 ms; a step's start, the
/// build directory's measure of a 25 GB target, 8.75 ms. 5 s is over 300
/// times the longest, for a far busier machine.
pub const IN_PROCESS: Duration = Duration::from_secs(5);

/// A watched part; dropping it, when the part is done, ends its watch.
#[derive(Debug)]
pub struct Watched {
    _done: Sender<()>,
}

/// Watch `part` for `deadline`: past it, fail the gate naming `part`
/// and end the process.
pub fn watch(part: impl Into<String>, deadline: Duration) -> Watched {
    watch_with(part, deadline, |line| {
        println!("{line}");
        std::process::exit(1);
    })
}

/// [`watch`], `overrun` given the failure line at the deadline.
pub fn watch_with(
    part: impl Into<String>,
    deadline: Duration,
    overrun: impl FnOnce(String) + Send + 'static,
) -> Watched {
    let (done, finished) = std::sync::mpsc::channel::<()>();
    let part = part.into();
    std::thread::spawn(move || {
        if finished.recv_timeout(deadline) == Err(RecvTimeoutError::Timeout) {
            overrun(format!(
                "FAIL gate: {part} did not finish within {deadline:?}; \
                 fix: find what holds it up (`ps`, the filesystem it reads), then rerun `cargo xtask gate`"
            ));
        }
    });
    Watched { _done: done }
}

/// `work`, watched as `part` for `deadline`.
pub fn watched<T>(part: impl Into<String>, deadline: Duration, work: impl FnOnce() -> T) -> T {
    let _watched = watch(part, deadline);
    work()
}

#[cfg(test)]
#[path = "gate_watchdog_tests.rs"]
mod tests;
