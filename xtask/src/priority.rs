//! Builds and checks yield to the gate. Outside `cargo xtask gate`, each
//! step xtask starts runs under `nice -n 10`, with `CARGO_BUILD_JOBS` set
//! to `[build] jobs` in .config/factory.toml (4 when unset). The gate's
//! steps run at normal priority on every core. Load from four
//! implementers' builds and page checks at once (load average 38 to 68
//! on 12 threads, 2026-10-25) made gates fail on timing.
//!
//! The sccache server keeps the niceness it starts with and serves every
//! worktree, so the outermost xtask starts it at its own priority before
//! any niced step can: `sccache` itself is never niced, and the
//! outermost xtask's first step asks it for its stats. Compiles through it run
//! at that priority; nice slows cargo, linkers, tests and browsers, and
//! the job cap bounds how many compiles run at once.
//!
//! Declared limits: plain `cargo` outside xtask is not reached; test
//! threads are not capped; `cargo metadata`, git and kill run as they are.

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

/// Set on niced steps, so an xtask a step runs, already at nice 10,
/// does not add 10 more.
pub const NICED_ENV: &str = "XTASK_NICED";

/// Never niced: see the module's note on its server.
const SCCACHE: &str = "sccache";

/// The command for a step's `program`, with the gate's or the others' priority.
pub fn command(program: &str) -> Result<Command, String> {
    let gate = in_gate(
        IN_GATE.load(Ordering::SeqCst),
        std::env::var(GATE_ENV).ok().as_deref(),
    );
    let niced = std::env::var(NICED_ENV).is_ok_and(|v| v == "1");
    let mut command = command_for(program, gate, niced);
    if !gate {
        command.env("CARGO_BUILD_JOBS", configured_jobs()?.to_string());
    }
    Ok(command)
}

/// Whether this process runs in the gate: it is the gate, or a gate's
/// step started it with [`GATE_ENV`] set to 1.
pub fn in_gate(is_gate: bool, gate_env: Option<&str>) -> bool {
    is_gate || gate_env == Some("1")
}

/// `program` in the gate: as is, marked with [`GATE_ENV`], with any
/// inherited job cap cleared. Outside it: under `nice -n 10`, marked
/// with [`NICED_ENV`], unless this xtask is already niced or the
/// program is sccache.
pub fn command_for(program: &str, in_gate: bool, niced: bool) -> Command {
    if in_gate {
        let mut command = Command::new(program);
        command.env(GATE_ENV, "1").env_remove("CARGO_BUILD_JOBS");
        return command;
    }
    // sccache may start its server, which keeps the niceness it starts
    // with and serves the gate too.
    if niced || program == SCCACHE {
        return Command::new(program);
    }
    let mut command = Command::new("nice");
    command.args(["-n", "10", program]).env(NICED_ENV, "1");
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
