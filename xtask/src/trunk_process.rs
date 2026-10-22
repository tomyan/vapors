//! The process side of [`Trunk`](crate::trunk::Trunk): its own process
//! group on Unix so the cargo build dies with it, both streams on one
//! channel, and the kill that reaches the group.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Both of the child's streams, line by line, on one channel. Lines are
/// split on `\n` and read as lossy UTF-8, so a stray byte cannot stop a
/// reader thread.
pub fn stream_lines(child: &mut Child) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    let streams: [Option<Box<dyn Read + Send>>; 2] = [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ];
    for stream in streams.into_iter().flatten() {
        let sender = sender.clone();
        thread::spawn(move || forward_lines(stream, &sender));
    }
    receiver
}

fn forward_lines(stream: Box<dyn Read + Send>, sender: &mpsc::Sender<String>) {
    for bytes in BufReader::new(stream).split(b'\n').map_while(Result::ok) {
        let line = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\r')
            .to_owned();
        if sender.send(line).is_err() {
            break;
        }
    }
}

#[cfg(unix)]
pub const ORPHAN_NOTE: &str = "";
#[cfg(not(unix))]
pub const ORPHAN_NOTE: &str = "; a cargo build trunk started may still be running";

#[cfg(unix)]
pub fn in_own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
pub fn in_own_process_group(_command: &mut Command) {}

/// SIGTERM to the group led by `pid`, through `kill` so no libc binding
/// is needed; a group that is already gone is not an error here.
#[cfg(unix)]
pub fn kill_process_group(pid: u32) {
    drop(
        Command::new("kill")
            .args(["-TERM", "--", &format!("-{pid}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    );
}

#[cfg(not(unix))]
pub fn kill_process_group(_pid: u32) {}
