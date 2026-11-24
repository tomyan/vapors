//! The lock tests' handle on their second process (`lock_child.rs`):
//! started with its action and lock directory, sequenced by the lines it
//! says and by its stdin, never by sleeping; killed if a test panics.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use crate::lock_child::{ACTION, DIR, WAIT_MS, WORKTREE};

/// How long the parent waits for a child's next line or its exit.
/// Measured 2026-11-22: the killed-holder test, one child started and
/// its `held` line read, took 12 to 36 ms in three runs at load 17 to
/// 25; 20 s is over 500 times that, for a machine far busier.
pub const WITHIN: Duration = Duration::from_secs(20);

/// A running child, as the parent sees it.
pub struct Child {
    process: std::process::Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    waits: Receiver<String>,
}

impl Child {
    /// Start a child taking `action` in `dir` as a run in `worktree`,
    /// each of its waits `wait`, with `env` set and no gate or quiet
    /// variable inherited unless `env` sets it.
    pub fn start(
        action: &str,
        dir: &Path,
        worktree: &str,
        wait: Duration,
        env: &[(&str, &str)],
    ) -> Self {
        let mut command = Command::new(std::env::current_exe().expect("this test binary"));
        command
            .args([
                "lock_child::lock_child",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env_remove(crate::priority::GATE_ENV)
            .env_remove(crate::quiet_wait::HOLDER_ENV)
            .envs([(ACTION, action), (WORKTREE, worktree)])
            .env(DIR, dir)
            .env(WAIT_MS, wait.as_millis().to_string())
            .envs(env.iter().copied())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut process = command.spawn().expect("the child started");
        let stdin = process.stdin.take();
        let (lines, waits) = said_lines(process.stdout.take().expect("its stdout"));
        Self {
            process,
            stdin,
            lines,
            waits,
        }
    }

    /// Its next line within [`WITHIN`], or none.
    pub fn next_line(&self) -> Option<String> {
        self.lines.recv_timeout(WITHIN).ok()
    }

    /// Its next line, which must be `line`.
    pub fn expect(&self, line: &str) {
        assert_eq!(
            self.next_line().as_deref(),
            Some(line),
            "the child's next line"
        );
    }

    /// Wait for it to say it waits for the gate lock held by `holder`, as
    /// `gate_lock::wait_for` says on its first try that finds it held.
    pub fn expect_waiting_on(&self, holder: &str) {
        let started = Instant::now();
        while let Some(left) = WITHIN.checked_sub(started.elapsed()) {
            match self.waits.recv_timeout(left) {
                Ok(line) if line.contains("waiting for the gate lock") && line.contains(holder) => {
                    return;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        panic!("the child did not say it waits for the gate lock held by {holder}");
    }

    /// Whether it has said nothing since its last line read.
    pub fn said_nothing(&self) -> bool {
        self.lines.try_recv().is_err()
    }

    /// Its line, if one has come, without waiting.
    pub fn line_now(&self) -> Option<String> {
        self.lines.try_recv().ok()
    }

    /// Send it one line on stdin.
    pub fn send(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("its stdin is open");
        writeln!(stdin, "{line}").expect("sent");
    }

    /// Close its stdin, so it lets go of what it holds, and wait for it
    /// to exit 0 within [`WITHIN`].
    pub fn release(mut self) {
        drop(self.stdin.take());
        let started = Instant::now();
        while started.elapsed() < WITHIN {
            if let Some(status) = self.process.try_wait().expect("its status") {
                assert!(status.success(), "the child exited {status}");
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the child did not exit within {WITHIN:?} of its stdin closing");
    }

    /// Kill it, as a gate killed mid-step, and reap it.
    pub fn kill(mut self) {
        self.process.kill().expect("killed");
        self.process.wait().expect("reaped");
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// The child's `child: ` lines, and the lines its waits print (`<who>:
/// waiting for ...`), read on a thread of their own.
fn said_lines(stdout: std::process::ChildStdout) -> (Receiver<String>, Receiver<String>) {
    let (said, lines) = std::sync::mpsc::channel();
    let (waited, waits) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(child) = line.strip_prefix("child: ") {
                let _ = said.send(child.to_owned());
            } else if line.contains(": waiting for ") {
                let _ = waited.send(line);
            }
        }
    });
    (lines, waits)
}

/// Wait until another holder has `gate.turn` in `dir`: poll its lock,
/// rather than sleep for a time a loaded box may not keep ([`WITHIN`] at
/// most).
pub fn turn_taken(dir: &Path) {
    let turn = crate::gate_lock::open(dir, crate::gate_lock::TURN).expect("turn");
    let started = Instant::now();
    while started.elapsed() < WITHIN {
        if turn.try_lock().is_err() {
            return;
        }
        let _ = turn.unlock();
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("nobody took gate.turn within {WITHIN:?}");
}
