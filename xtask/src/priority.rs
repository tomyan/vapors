//! Builds and checks yield to the gate. Outside `cargo xtask gate`, each
//! step xtask starts runs under `nice -n 10`, with `CARGO_BUILD_JOBS` set
//! to `[build] jobs` in .config/factory.toml (4 when unset). The gate's
//! steps run at normal priority on every core. Load from four
//! implementers' builds and page checks at once (load average 38 to 68
//! on 12 threads, 2026-10-25) made gates fail on timing.
//!
//! Declared limit: compiles through sccache run in its server, at the
//! server's priority, so nice slows cargo, linkers, tests and browsers,
//! not those compiles. The job cap bounds how many run at once.

use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::Config;

static IN_GATE: AtomicBool = AtomicBool::new(false);

/// Set on the gate's steps, so an xtask a step runs (`cargo xtask lint`)
/// knows it is in the gate too.
pub const GATE_ENV: &str = "XTASK_GATE";

/// The cap when `[build] jobs` is unset: a third of the dev machine's 12
/// threads, so about three implementers can build at once.
const DEFAULT_JOBS: u32 = 4;

/// Mark this process as the gate: its steps run at normal priority.
pub fn enter_gate() {
    IN_GATE.store(true, Ordering::SeqCst);
}

/// The command for a step's `program`, with the gate's or the others' priority.
pub fn command(program: &str) -> Result<Command, String> {
    let in_gate = in_gate(
        IN_GATE.load(Ordering::SeqCst),
        std::env::var(GATE_ENV).ok().as_deref(),
    );
    let mut command = command_for(program, in_gate);
    if !in_gate {
        command.env("CARGO_BUILD_JOBS", configured_jobs()?.to_string());
    }
    Ok(command)
}

/// Whether this process runs in the gate: it is the gate, or a gate's
/// step started it with [`GATE_ENV`] set to 1.
pub fn in_gate(is_gate: bool, gate_env: Option<&str>) -> bool {
    is_gate || gate_env == Some("1")
}

/// `program` as is in the gate, marked with [`GATE_ENV`], else under
/// `nice -n 10`.
pub fn command_for(program: &str, in_gate: bool) -> Command {
    if in_gate {
        let mut command = Command::new(program);
        command.env(GATE_ENV, "1");
        return command;
    }
    let mut command = Command::new("nice");
    command.args(["-n", "10", program]);
    command
}

fn configured_jobs() -> Result<u32, String> {
    static JOBS: OnceLock<Result<u32, String>> = OnceLock::new();
    // From the workspace root, xtask's parent, wherever the step runs.
    let xtask = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = xtask.parent().unwrap_or(xtask);
    JOBS.get_or_init(|| {
        Config::load_from(root)
            .map_err(|e| e.to_string())
            .and_then(|c| jobs(&c))
    })
    .clone()
}

/// `[build] jobs`, a positive number, or [`DEFAULT_JOBS`] when unset.
pub fn jobs(config: &Config) -> Result<u32, String> {
    let Some(text) = config.get("build.jobs") else {
        return Ok(DEFAULT_JOBS);
    };
    match text.parse::<u32>() {
        Ok(jobs) if jobs > 0 => Ok(jobs),
        _ => Err(format!(
            "[build] jobs is `{text}`, not a positive number; fix: .config/factory.toml"
        )),
    }
}

#[cfg(test)]
#[path = "priority_tests.rs"]
mod tests;
