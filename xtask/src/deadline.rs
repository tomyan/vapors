//! Running a child process that must finish by a deadline.

use std::io::Read;
use std::process::{Command, Stdio};
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
    TimedOut,
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
            return Ok(Finished::TimedOut);
        }
        thread::sleep(POLL);
    }
}

fn read_in_background<R: Read + Send + 'static>(mut reader: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut text = String::new();
        let _ = reader.read_to_string(&mut text);
        text
    })
}

fn join_outputs(
    stdout: Option<thread::JoinHandle<String>>,
    stderr: Option<thread::JoinHandle<String>>,
) -> String {
    let mut text = String::new();
    for handle in [stdout, stderr].into_iter().flatten() {
        text.push_str(&handle.join().unwrap_or_default());
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
            Finished::TimedOut => panic!("should not time out"),
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
        assert!(matches!(finished, Finished::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
