//! Running a child process that must finish by a deadline.

use std::collections::VecDeque;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// How a child process ended.
pub enum Finished {
    /// It exited on its own.
    Exited {
        /// Exit status zero.
        success: bool,
        /// Its combined output.
        output: String,
    },
    /// It was killed at the deadline.
    TimedOut {
        /// Its combined output up to the kill.
        output: String,
    },
}

const POLL: Duration = Duration::from_millis(50);

/// Spawn `command`, wait at most `deadline`, kill it if it is still
/// running. The child's output is captured in both cases.
pub fn run_with_deadline(mut command: Command, deadline: Duration) -> Result<Finished, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            format!(
                "could not start {}: {e}",
                command.get_program().to_string_lossy()
            )
        })?;
    let stdout = child.stdout.take().map(read_in_background);
    let stderr = child.stderr.take().map(read_in_background);
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let output = join_outputs(stdout, stderr);
            return Ok(Finished::Exited {
                success: status.success(),
                output,
            });
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            // Not joined: a grandchild may still hold the pipe open.
            let output = snapshot(stdout.as_ref(), stderr.as_ref());
            return Ok(Finished::TimedOut { output });
        }
        thread::sleep(POLL);
    }
}

/// Each stream keeps its last 64 KiB: enough for any failure's tail
/// (binstall's longest error is under 2 KiB), bounded for a chatty child.
pub const KEPT_BYTES: usize = 64 * 1024;

/// How long the readers may take after the child exits. Without a
/// grandchild holding the pipe they finish within milliseconds.
pub const JOIN_DEADLINE: Duration = Duration::from_secs(2);

const DROPPED: &str = "[earlier output dropped; the last 64 KiB is kept]\n";
const STILL_OPEN: &str = "\n[the output pipe was still held open by a child process after the exit; reading stopped after 2 s]\n";

/// The last bytes read, and whether earlier ones were dropped.
#[derive(Default)]
struct Ring {
    bytes: VecDeque<u8>,
    dropped: bool,
}

impl Ring {
    fn push(&mut self, chunk: &[u8]) {
        self.bytes.extend(chunk);
        let excess = self.bytes.len().saturating_sub(KEPT_BYTES);
        if excess > 0 {
            self.bytes.drain(..excess);
            self.dropped = true;
        }
    }

    /// The text, decoded once so no character is split between reads; a
    /// character the drop cut in half is left out.
    fn text(&self) -> String {
        let bytes: Vec<u8> = self.bytes.iter().copied().collect();
        if !self.dropped {
            return String::from_utf8_lossy(&bytes).into_owned();
        }
        let start = bytes
            .iter()
            .position(|b| b & 0xC0 != 0x80)
            .unwrap_or(bytes.len());
        format!("{DROPPED}{}", String::from_utf8_lossy(&bytes[start..]))
    }
}

/// A reader thread and what it has read so far.
struct Collected {
    ring: Arc<Mutex<Ring>>,
    thread: thread::JoinHandle<()>,
}

fn read_in_background<R: Read + Send + 'static>(mut reader: R) -> Collected {
    let ring = Arc::new(Mutex::new(Ring::default()));
    let sink = Arc::clone(&ring);
    let thread = thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(read) = reader.read(&mut chunk) {
            if read == 0 {
                break;
            }
            sink.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(&chunk[..read]);
        }
    });
    Collected { ring, thread }
}

/// The child exited: wait for the readers, by [`JOIN_DEADLINE`], then
/// take what they read, saying so when a pipe was still open.
fn join_outputs(stdout: Option<Collected>, stderr: Option<Collected>) -> String {
    let collected: Vec<Collected> = [stdout, stderr].into_iter().flatten().collect();
    let started = Instant::now();
    let finished = |all: &[Collected]| all.iter().all(|reader| reader.thread.is_finished());
    while !finished(&collected) && started.elapsed() < JOIN_DEADLINE {
        thread::sleep(POLL);
    }
    let mut text = snapshot(collected.first(), collected.get(1));
    if !finished(&collected) {
        text.push_str(STILL_OPEN);
    }
    text
}

fn snapshot(stdout: Option<&Collected>, stderr: Option<&Collected>) -> String {
    let mut text = String::new();
    for reader in [stdout, stderr].into_iter().flatten() {
        text.push_str(
            &reader
                .ring
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .text(),
        );
    }
    text
}

#[cfg(test)]
#[path = "deadline_tests.rs"]
mod tests;
