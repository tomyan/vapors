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
        None => Err(kill_tree(&mut child, deadline)),
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

/// Kill the child's process group and the group of each of its
/// descendants: the gate puts each step in a group of its own
/// (`command.rs`) and enforced the step's deadline itself, so the steps
/// must go too. What was killed and what could not be, for the log.
/// Declared limit: a process started between the scan and the kill is
/// missed.
fn kill_tree(child: &mut Child, deadline: Duration) -> String {
    let mut groups = vec![child.id()];
    for group in crate::queue_process_tree::descendant_groups(child.id()) {
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    let failed: Vec<String> = groups
        .iter()
        .filter_map(|group| crate::command::kill_group_id(*group).err())
        .collect();
    let _ = child.kill();
    let _ = child.wait();
    let listed: Vec<String> = groups.iter().map(u32::to_string).collect();
    let said = if failed.is_empty() {
        format!("killed its process groups ({})", listed.join(", "))
    } else {
        format!(
            "killed what it could of its process groups ({}); not killed: {}",
            listed.join(", "),
            failed.join("; ")
        )
    };
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
