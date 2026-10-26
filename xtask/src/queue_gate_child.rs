//! `gate --pr` as the queue's child: in a process group of its own, so
//! the kill at its deadline also ends the `gate` it started (which holds
//! the gate lock); its output in a file beside the queue's log, so the
//! last lines can say why when it posted nothing.

use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::gate_pr::Ended;
use crate::queue_ops::Gated;

/// How long a gate run by the queue may take: the gate lock's 40 min
/// wait (`gate_lock::WAIT`) and a touche gate's measured 4.5 to 8 min,
/// with room. Killed at the deadline, and the kill is logged.
pub const GATE_DEADLINE: Duration = Duration::from_mins(60);

/// Run `command` within `deadline`, its output to `out`.
///
/// # Errors
///
/// It could not start, or did not finish in time (it is then killed).
pub fn run(mut command: Command, deadline: Duration, out: &Path) -> Result<Gated, String> {
    use std::os::unix::process::CommandExt;
    let named = |e: std::io::Error| format!("{}: {e}", out.display());
    let file = std::fs::File::create(out).map_err(named)?;
    let errors = file.try_clone().map_err(named)?;
    let mut child = command
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(errors)
        .spawn()
        .map_err(|e| format!("could not start the gate: {e}"))?;
    match wait(&mut child, deadline)? {
        Some(status) => Ok(Gated {
            ended: ended(status)?,
            tail: tail(out)?,
        }),
        None => Err(kill_group(&mut child, deadline)),
    }
}

/// The child's exit within `deadline`, or none when it is still running.
fn wait(child: &mut Child, deadline: Duration) -> Result<Option<ExitStatus>, String> {
    let started = Instant::now();
    while started.elapsed() <= deadline {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("waiting for the gate: {e}"))?
        {
            return Ok(Some(status));
        }
        std::thread::sleep(POLL);
    }
    Ok(None)
}

/// How often the child is polled; a gate takes minutes.
const POLL: Duration = Duration::from_millis(200);

/// Kill the child's whole process group (no libc, so by `kill`); what
/// happened, for the log.
fn kill_group(child: &mut Child, deadline: Duration) -> String {
    let group = format!("-{}", child.id());
    let killed = Command::new("kill").args(["-KILL", "--", &group]).status();
    let said = match killed {
        Ok(status) if status.success() => "killed with its process group".to_owned(),
        other => {
            let _ = child.kill();
            format!("its process group could not be killed ({other:?}), so the gate alone was")
        }
    };
    let _ = child.wait();
    format!("the gate did not finish within {deadline:?}; {said}")
}

/// An exit status as a code or the signal that ended it.
fn ended(status: ExitStatus) -> Result<Ended, String> {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => Ok(Ended::Code(code)),
        (None, Some(signal)) => Ok(Ended::Signal(signal)),
        (None, None) => Err("the gate ended with neither a code nor a signal".into()),
    }
}

/// How many of the gate's last lines the log gets.
const TAIL_LINES: usize = 10;

fn tail(out: &Path) -> Result<String, String> {
    let bytes = std::fs::read(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    Ok(lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n"))
}

#[cfg(test)]
#[path = "queue_gate_child_tests.rs"]
mod tests;
