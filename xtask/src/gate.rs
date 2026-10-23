//! `cargo xtask gate`: every command CI's jobs run, from this worktree,
//! in the order `[gate]` in `.config/factory.toml` gives (fastest
//! first), stopping at the first failure. It ends with
//! `gate: pass <HEAD sha>`, marked dirty when the tree has changes, so a
//! pull request can carry a local result while CI cannot run.
//!
//! One answer: before running, the list must match the commands
//! `.github/workflows/ci.yml` runs, setup aside (`[gate.setup]`
//! prefixes, each with its reason), or the gate refuses naming each
//! difference.

use std::process::{Command, ExitCode};

use crate::config::Config;
use crate::gate_workflow::{agree, without_env};
use crate::report::Reporter;
use crate::steps::Step;

/// The workflow the gate must agree with.
pub const WORKFLOW: &str = ".github/workflows/ci.yml";

/// Run the gate.
pub fn run(reporter: &Reporter) -> ExitCode {
    match prepare() {
        Ok(steps) => finish(crate::ci::run(&steps, reporter)),
        Err(why) => {
            println!("FAIL gate: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The steps, once the list is known to match CI's.
fn prepare() -> Result<Vec<Step>, String> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let gate = gate_commands(&config);
    if gate.is_empty() {
        return Err(
            "no [gate] commands in .config/factory.toml; fix: list what ci.yml runs".to_owned(),
        );
    }
    let workflow = std::fs::read_to_string(WORKFLOW)
        .map_err(|e| format!("{WORKFLOW}: {e}; run from the workspace root"))?;
    agree(&gate, &workflow, &setup_prefixes(&config))
        .map_err(|differences| format!("the gate and {WORKFLOW} differ:\n{differences}\nfix: make [gate] in .config/factory.toml and {WORKFLOW} run the same commands"))?;
    Ok(gate.iter().map(|command| step_for(command)).collect())
}

/// `gate: pass <sha>` after a pass.
fn finish(code: ExitCode) -> ExitCode {
    if code == ExitCode::SUCCESS {
        println!("gate: pass {}", head());
    }
    code
}

/// `[gate]` commands, in key order.
#[must_use]
pub fn gate_commands(config: &Config) -> Vec<String> {
    config
        .entries_under("gate.")
        .into_iter()
        .filter(|(key, _)| !key.contains('.'))
        .map(|(_, command)| command)
        .collect()
}

/// `[gate.setup]` keys: command prefixes that are the runner's setup.
#[must_use]
pub fn setup_prefixes(config: &Config) -> Vec<String> {
    config.entries_under("gate.setup.").into_keys().collect()
}

/// A step running `command` in `sh`, named by the command without its
/// environment, cut to 60 characters, so two steps never share a name.
#[must_use]
pub fn step_for(command: &str) -> Step {
    Step {
        name: without_env(command).chars().take(60).collect(),
        program: "sh".to_owned(),
        args: vec!["-c".to_owned(), command.to_owned()],
        fix: "fix it, then rerun `cargo xtask gate`".to_owned(),
    }
}

/// HEAD's SHA, ` (dirty)` when the tree has changes. git's hook
/// variables are stripped (CLAUDE.md).
fn head() -> String {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
    };
    let sha = git(&["rev-parse", "HEAD"]).map_or_else(
        |e| format!("(no git: {e})"),
        |o| String::from_utf8_lossy(&o.stdout).trim().to_owned(),
    );
    let dirty = git(&["status", "--porcelain"]).map_or(true, |o| !o.stdout.is_empty());
    if dirty { format!("{sha} (dirty)") } else { sha }
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
