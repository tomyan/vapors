//! Running a child process that must finish by a deadline.

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

/// A reader thread and the text it has read so far.
struct Collected {
    text: Arc<Mutex<String>>,
    thread: thread::JoinHandle<()>,
}

fn read_in_background<R: Read + Send + 'static>(mut reader: R) -> Collected {
    let text = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&text);
    let thread = thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(read) = reader.read(&mut chunk) {
            if read == 0 {
                break;
            }
            let mut text = sink.lock().unwrap_or_else(PoisonError::into_inner);
            text.push_str(&String::from_utf8_lossy(&chunk[..read]));
        }
    });
    Collected { text, thread }
}

/// The child exited: wait for the readers, then take everything.
fn join_outputs(stdout: Option<Collected>, stderr: Option<Collected>) -> String {
    let collected: Vec<Collected> = [stdout, stderr].into_iter().flatten().collect();
    for reader in &collected {
        while !reader.thread.is_finished() {
            thread::sleep(POLL);
        }
    }
    snapshot(collected.first(), collected.get(1))
}

fn snapshot(stdout: Option<&Collected>, stderr: Option<&Collected>) -> String {
    let mut text = String::new();
    for reader in [stdout, stderr].into_iter().flatten() {
        text.push_str(&reader.text.lock().unwrap_or_else(PoisonError::into_inner));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_command_exits_with_its_output() {
        // Given
        let mut command = Command::new("sh");
        command.args(["-c", "echo hello"]);

        // When
        let finished = run_with_deadline(command, Duration::from_secs(5)).expect("spawns");

        // Then
        match finished {
            Finished::Exited { success, output } => {
                assert!(success);
                assert_eq!(output.trim(), "hello");
            }
            Finished::TimedOut { .. } => panic!("should not time out"),
        }
    }

    #[test]
    fn a_slow_command_is_killed_at_the_deadline() {
        // Given
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 10"]);

        // When
        let started = Instant::now();
        let finished = run_with_deadline(command, Duration::from_millis(200)).expect("spawns");

        // Then
        assert!(matches!(finished, Finished::TimedOut { .. }));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_command_killed_at_the_deadline_keeps_its_output_so_far() {
        // Given: the shell's `sleep` child keeps the pipe open past the kill.
        let mut command = Command::new("sh");
        command.args(["-c", "echo partial; sleep 10"]);

        // When
        let started = Instant::now();
        let finished = run_with_deadline(command, Duration::from_millis(500)).expect("spawns");

        // Then
        let Finished::TimedOut { output } = finished else {
            panic!("should time out");
        };
        assert_eq!(output.trim(), "partial");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
