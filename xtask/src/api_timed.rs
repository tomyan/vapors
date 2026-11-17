//! A child process's output within a deadline, its stdout and stderr
//! kept apart (review of slice 4, M3): `deadline::run_with_deadline`
//! joins the two, which a git or gh call whose stdout is its answer
//! cannot use. Each stream keeps at most [`KEPT_BYTES`], so a hostile
//! branch's output cannot grow the host's memory.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The most bytes kept of each stream: a slice's numstat is a few KiB.
pub const KEPT_BYTES: u64 = 16 * 1024 * 1024;

/// How a child that ended in time ended.
pub struct Ended {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// `command` run to its end, or killed at `deadline`.
///
/// # Errors
///
/// It did not start, or it passed `deadline` and was killed, naming it.
pub fn within(mut command: Command, deadline: Duration) -> Result<Ended, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {program}: {e}"))?;
    let stdout = child.stdout.take().map(kept);
    let stderr = child.stderr.take().map(kept);
    let started = Instant::now();
    while child.try_wait().map_err(|e| e.to_string())?.is_none() {
        if started.elapsed() >= deadline {
            let _ = child.kill(); // past its deadline; the error says so
            let _ = child.wait();
            return Err(format!(
                "{program} passed its deadline of {deadline:?} and was killed"
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    Ok(Ended {
        status,
        stdout: joined(stdout),
        stderr: joined(stderr),
    })
}

fn kept(stream: impl Read + Send + 'static) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stream.take(KEPT_BYTES).read_to_end(&mut bytes); // what was read is what is kept
        bytes
    })
}

fn joined(reader: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "api_timed_tests.rs"]
mod tests;
