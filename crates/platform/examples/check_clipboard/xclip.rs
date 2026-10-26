//! The other program on the clipboard: xclip, set as another program
//! would, and its serving child found and signalled.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// xclip forks and serves the selection; its parent exits at once
/// (under 10 ms here). 5 s is the deadline for it to say it has.
const XCLIP_DEADLINE: Duration = Duration::from_secs(5);

/// `xclip -selection clipboard <args>`, given a file or bytes on stdin;
/// refused naming the fix when xclip is missing or does not finish.
pub fn xclip(args: &[&str], file: Option<&Path>, stdin: Option<&[u8]>) -> Result<(), String> {
    let mut command = Command::new("xclip");
    command.args(["-selection", "clipboard"]).args(args);
    if let Some(file) = file {
        command.arg(file);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| format!("xclip: {e}; fix: install xclip (apt install xclip)"))?;
    if let (Some(bytes), Some(mut pipe)) = (stdin, child.stdin.take()) {
        std::io::Write::write_all(&mut pipe, bytes).map_err(|e| format!("xclip's stdin: {e}"))?;
    }
    drop(child.stdin.take());
    wait(&mut child)
}

fn wait(child: &mut std::process::Child) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < XCLIP_DEADLINE {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("xclip exited {status}"))
            };
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    Err(format!("xclip did not finish within {XCLIP_DEADLINE:?}"))
}

pub fn newest_xclip() -> Result<String, String> {
    let out = Command::new("pgrep")
        .args(["-n", "-x", "xclip"])
        .output()
        .map_err(|e| format!("pgrep: {e}"))?;
    let pid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if pid.is_empty() {
        Err("no xclip serving the clipboard to stop".to_owned())
    } else {
        Ok(pid)
    }
}

pub fn signal(pid: &str, name: &str) -> Result<(), String> {
    let status = Command::new("kill")
        .args([&format!("-{name}"), pid])
        .status()
        .map_err(|e| format!("kill: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("kill -{name} {pid}: {status}"))
    }
}
