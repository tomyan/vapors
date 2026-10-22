//! The trunk child process `cargo xtask dev` drives: spawned in its own
//! process group so the cargo build it starts dies with it, read line by
//! line on one channel, waited on until its server is up, and killed
//! whenever the handle is dropped, so no failure path can orphan it.
//! Only a terminal Ctrl-C stops a running loop cleanly; see docs/web.md.

use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::report::Reporter;
use crate::stream::without_timestamp;
use crate::trunk_process::{ORPHAN_NOTE, in_own_process_group, kill_process_group, stream_lines};

/// The line trunk prints once its server is up. A TCP connect to the
/// port is the second signal, so a changed line cannot cost the deadline.
pub const READY_LINE: &str = "server listening at:";

/// How many of trunk's last lines a failure message carries.
const RECENT_LINES: usize = 40;

/// A running trunk. Dropping it kills the process group.
pub struct Trunk {
    child: Child,
    lines: Receiver<String>,
    recent: VecDeque<String>,
}

impl Trunk {
    /// Start `command` with both streams captured, in its own process
    /// group on Unix.
    pub fn spawn(mut command: Command) -> Result<Self, String> {
        let program = command.get_program().to_string_lossy().into_owned();
        in_own_process_group(&mut command);
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not start {program}: {e}"))?;
        let lines = stream_lines(&mut child);
        Ok(Self {
            child,
            lines,
            recent: VecDeque::new(),
        })
    }

    /// The child's process id, for the tests that check it is gone.
    #[cfg(test)]
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Print trunk's lines until its ready line or until `port` accepts
    /// a connection, within `deadline`; kill it when the deadline passes.
    pub fn wait_until_ready(
        &mut self,
        port: u16,
        reporter: &Reporter,
        deadline: Duration,
    ) -> Result<(), String> {
        let started = Instant::now();
        while started.elapsed() < deadline {
            match self.next_line(Duration::from_millis(100)) {
                Ok(Some(line)) => {
                    reporter.stream("trunk", &line);
                    if line.contains(READY_LINE) {
                        return Ok(());
                    }
                }
                Ok(None) => {}
                Err(Exited) => return Err(self.exited_early()),
            }
            if port_accepts(port) {
                return Ok(());
            }
        }
        self.kill_group();
        Err(self.deadline_passed(port, deadline))
    }

    /// Stream rebuild lines until trunk exits; a non-zero exit is a failure.
    pub fn follow(&mut self, reporter: &Reporter) -> Result<(), String> {
        while let Ok(Some(line)) = self.next_line(Duration::from_secs(1)) {
            reporter.stream("trunk", &line);
        }
        let status = self.child.wait().map_err(|e| format!("trunk: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("trunk exited with {status}{}", self.recent_lines()))
        }
    }

    /// The next line within `wait`: `Ok(None)` when none came, `Err`
    /// once both streams have closed.
    fn next_line(&mut self, wait: Duration) -> Result<Option<String>, Exited> {
        match self.lines.recv_timeout(wait) {
            Ok(line) => {
                self.remember(&line);
                Ok(Some(line))
            }
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(Exited),
        }
    }

    fn remember(&mut self, line: &str) {
        if self.recent.len() == RECENT_LINES {
            self.recent.pop_front();
        }
        self.recent.push_back(without_timestamp(line).to_owned());
    }

    /// The last lines for a failure message, or a note that there were none.
    fn recent_lines(&self) -> String {
        if self.recent.is_empty() {
            return "; trunk printed nothing".to_owned();
        }
        let mut text = format!("; its last {} lines:", self.recent.len());
        for line in &self.recent {
            text.push_str("\n     ");
            text.push_str(line);
        }
        text
    }

    fn exited_early(&mut self) -> String {
        let status = self
            .child
            .wait()
            .map_or_else(|e| e.to_string(), |s| s.to_string());
        format!(
            "trunk exited before its server was up ({status}){}",
            self.recent_lines()
        )
    }

    fn deadline_passed(&self, port: u16, deadline: Duration) -> String {
        format!(
            "trunk did not print `{READY_LINE}` or open 127.0.0.1:{port} within {}s; fix: run `trunk serve` in the package directory to see why{}{}",
            deadline.as_secs(),
            self.recent_lines(),
            ORPHAN_NOTE
        )
    }

    /// Kill trunk and, on Unix, the cargo build in its process group.
    fn kill_group(&mut self) {
        kill_process_group(self.child.id());
        drop(self.child.kill());
        drop(self.child.wait());
    }
}

impl Drop for Trunk {
    fn drop(&mut self) {
        self.kill_group();
    }
}

/// Both streams have closed: trunk has exited.
struct Exited;

/// Whether something accepts connections on the loopback port.
fn port_accepts(port: u16) -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&address, Duration::from_millis(50)).is_ok()
}

#[cfg(test)]
#[path = "trunk_tests.rs"]
mod tests;
