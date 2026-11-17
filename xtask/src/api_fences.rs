//! The sandbox's fences, proven before every session
//! (`docs/plans/api-builder.md`). Each probe is an escape: it is run
//! outside first, where it must succeed, so a probe that could not see
//! its escape fails the run rather than passing it; then inside, where
//! it must fail. The way in is probed too: a request through the relay
//! must reach the harness server.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};

/// How long a probe may run: the network probe's curl gives up at 15 s,
/// and the rest take milliseconds; 30 s is reached only by a hang, which
/// fails the probe.
pub const PROBE_DEADLINE: Duration = Duration::from_secs(30);

/// How a probe's command ended: exited 0 or not (a kill at the deadline
/// is not), and what it said.
pub struct Ran {
    pub success: bool,
    pub said: String,
}

/// One escape, as a shell command, and what shows it got out.
pub struct Probe {
    pub name: &'static str,
    pub script: String,
    /// Where an escape that exits 0 inside still lands nowhere real:
    /// the file it writes, which must not exist outside afterwards.
    pub lands_at: Option<PathBuf>,
}

/// What the probes need to know of the run.
pub struct Escapes<'a> {
    /// A file in the builder's directory, hidden inside by the tmpfs home.
    pub hidden: PathBuf,
    /// A directory bound read-only inside: the main repository's objects.
    pub read_only: &'a Path,
    pub key_file: &'a Path,
    pub clone_dir: &'a Path,
}

/// The escapes, each with a control run outside.
pub fn probes(escapes: &Escapes<'_>) -> Vec<Probe> {
    let quoted = |path: &Path| format!("'{}'", path.display());
    let read_only = escapes.read_only.join("api-build-probe");
    vec![
        Probe {
            name: "write outside the clone, through the hidden home",
            script: format!("echo escaped > {0} && test -s {0}", quoted(&escapes.hidden)),
            lands_at: Some(escapes.hidden.clone()),
        },
        Probe {
            name: "write into the main repository's objects",
            script: format!("touch {0} && rm {0}", quoted(&read_only)),
            lands_at: None,
        },
        Probe {
            name: "reach the network directly",
            script: "curl -q -sS -I --max-time 15 -o /dev/null https://api.anthropic.com/api/hello"
                .into(),
            lands_at: None,
        },
        Probe {
            name: "read the key",
            script: format!("test -r {}", quoted(escapes.key_file)),
            lands_at: None,
        },
        Probe {
            name: "see a process outside",
            script: format!("test -e /proc/{}", std::process::id()),
            lands_at: None,
        },
        Probe {
            name: "push",
            script: format!(
                "git -C {} push --dry-run --quiet origin HEAD:refs/heads/api-build-probe",
                quoted(escapes.clone_dir)
            ),
            lands_at: None,
        },
    ]
}

/// Each probe run outside, then inside through `inside`; the lines that
/// say each fence held.
///
/// # Errors
///
/// A probe whose control failed outside (it cannot show an escape), or
/// one that escaped inside, naming it and its output.
pub fn prove(
    probes: &[Probe],
    inside: &dyn Fn(&str) -> Result<Ran, String>,
) -> Result<Vec<String>, String> {
    let (held, broken): (Vec<_>, Vec<_>) = probes
        .iter()
        .map(|probe| checked(probe, inside))
        .partition(Result::is_ok);
    let broken: Vec<String> = broken.into_iter().filter_map(Result::err).collect();
    if broken.is_empty() {
        return Ok(held.into_iter().filter_map(Result::ok).collect());
    }
    Err(broken.join("; "))
}

/// One probe's control outside, then the probe inside.
fn checked(probe: &Probe, inside: &dyn Fn(&str) -> Result<Ran, String>) -> Result<String, String> {
    control(probe)?;
    let output = inside(&probe.script)?;
    if escaped(probe, &output) {
        if let Some(path) = &probe.lands_at {
            let _ = std::fs::remove_file(path); // what escaped is cleared; the run fails anyway
        }
        return Err(format!(
            "fence broken, {}: inside the sandbox `{}` succeeded ({}); the sandbox's arguments \
             are api_sandbox.rs; reproduce with `cargo xtask api-fences <name>`",
            probe.name, probe.script, output.said
        ));
    }
    Ok(format!("fence held: {}", probe.name))
}

/// The probe outside, where it must succeed; anything it wrote removed.
fn control(probe: &Probe) -> Result<(), String> {
    let output = shell(&probe.script)?;
    if let Some(path) = &probe.lands_at {
        std::fs::remove_file(path).map_err(|e| format!("removing {}: {e}", path.display()))?;
    }
    if output.success {
        return Ok(());
    }
    Err(format!(
        "probe cannot show an escape, {}: outside the sandbox `{}` failed too ({}); fix what \
         stops it outside (the network, the key file, git's credentials) and run again",
        probe.name, probe.script, output.said
    ))
}

fn escaped(probe: &Probe, output: &Ran) -> bool {
    match &probe.lands_at {
        Some(path) => path.exists(),
        None => output.success,
    }
}

/// `script` run by bash outside, git's hook variables stripped.
fn shell(script: &str) -> Result<Ran, String> {
    let mut command = Command::new("bash");
    command.args(["-c", script]);
    for variable in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        command.env_remove(variable);
    }
    ran(command)
}

/// `command` run to [`PROBE_DEADLINE`].
///
/// # Errors
///
/// It cannot start.
pub fn ran(command: Command) -> Result<Ran, String> {
    Ok(match run_with_deadline(command, PROBE_DEADLINE)? {
        Finished::Exited { success, output } => Ran {
            success,
            said: output.trim().to_owned(),
        },
        Finished::TimedOut { output } => Ran {
            success: false,
            said: format!("killed at {PROBE_DEADLINE:?}: {}", output.trim()),
        },
    })
}

#[cfg(test)]
#[path = "api_fences_tests.rs"]
mod tests;
