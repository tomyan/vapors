//! Stopping a child process politely: SIGTERM, a deadline, then
//! SIGKILL. A process killed outright cannot clean up; Xvfb, for one,
//! leaves its `/tmp/.X<n>-lock` and socket behind.

use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

/// How the child ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Stopped {
    /// It exited after SIGTERM, within the deadline.
    Exited,
    /// It was still running at the deadline and was killed.
    Killed,
}

/// Send SIGTERM to `child`, wait up to `deadline` for it to exit, then
/// SIGKILL it.
///
/// # Errors
///
/// When `kill` cannot be run or refuses, or the child cannot be waited
/// on; the message names what failed.
pub fn terminate(child: &mut Child, deadline: Duration) -> Result<Stopped, String> {
    send_term(child.id())?;
    let started = Instant::now();
    while started.elapsed() < deadline {
        if child
            .try_wait()
            .map_err(|e| format!("waiting for process {}: {e}", child.id()))?
            .is_some()
        {
            return Ok(Stopped::Exited);
        }
        thread::sleep(Duration::from_millis(10));
    }
    child
        .kill()
        .map_err(|e| format!("killing process {}: {e}", child.id()))?;
    child
        .wait()
        .map_err(|e| format!("waiting for process {}: {e}", child.id()))?;
    Ok(Stopped::Killed)
}

/// SIGTERM through `kill(1)`, since the crate allows no `unsafe`.
fn send_term(pid: u32) -> Result<(), String> {
    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .map_err(|e| format!("could not run `kill -TERM {pid}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`kill -TERM {pid}` exited with {status}"))
    }
}

#[cfg(test)]
#[path = "stop_tests.rs"]
mod tests;
