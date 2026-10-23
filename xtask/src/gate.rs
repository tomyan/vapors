//! `cargo xtask gate`: every `run` step `.github/workflows/ci.yml` runs,
//! from this worktree, as CI runs it: in file order, each script by
//! `bash -eo pipefail -c` with the environment CI gives it
//! (`gate_plan`), stopping at the first failure. The runner's setup is
//! left out by exact line (`[gate.setup]` in `.config/factory.toml`,
//! each with its reason). It ends with `gate: pass <HEAD sha>`, marked
//! dirty when the tree has changes; HEAD, the tree and each pinned
//! sibling (`[gate.pinned]`: a checkout that must be clean at the
//! commit its pin file names) are read before and after, and any
//! change, or an unreadable state, fails the gate.

use std::collections::BTreeSet;
use std::process::{Command, ExitCode};

use crate::config::Config;
use crate::gate_plan::{PlannedStep, plan};
use crate::report::Reporter;
use crate::steps::Step;

/// The workflow the gate runs.
pub const WORKFLOW: &str = ".github/workflows/ci.yml";

/// Run the gate.
pub fn run(reporter: &Reporter) -> ExitCode {
    let prepared = Config::load()
        .map_err(|e| e.to_string())
        .and_then(|config| prepare(&config));
    let (steps, before, siblings) = match prepared {
        Ok(prepared) => prepared,
        Err(why) => return refuse(&why),
    };
    let code = crate::ci::run(&steps, reporter);
    if code != ExitCode::SUCCESS {
        return code;
    }
    match state(&siblings) {
        Ok(after) if after == before => {
            println!("gate: pass {}", before.head);
            code
        }
        Ok(after) => refuse(&format!(
            "the checkout changed during the gate: before {before:?}, after {after:?}; fix: rerun on a quiet worktree"
        )),
        Err(why) => refuse(&why),
    }
}

fn refuse(why: &str) -> ExitCode {
    println!("FAIL gate: {why}");
    ExitCode::FAILURE
}

/// Pinned sibling checkouts: the directory, and the file naming its pin.
type Pinned = Vec<(String, String)>;

/// The steps, the state before, and the pinned siblings.
fn prepare(config: &Config) -> Result<(Vec<Step>, State, Pinned), String> {
    let workflow = std::fs::read_to_string(WORKFLOW)
        .map_err(|e| format!("{WORKFLOW}: {e}; run from the workspace root"))?;
    let planned =
        plan(&workflow, &setup_lines(config)).map_err(|why| format!("{WORKFLOW}: {why}"))?;
    let siblings: Pinned = config.entries_under("gate.pinned.").into_iter().collect();
    let before = state(&siblings)?;
    Ok((planned.iter().map(step_for).collect(), before, siblings))
}

/// `[gate.setup]` keys: the runner's setup, exact lines.
#[must_use]
pub fn setup_lines(config: &Config) -> BTreeSet<String> {
    config.entries_under("gate.setup.").into_keys().collect()
}

/// A step running `planned`'s script as CI does.
#[must_use]
pub fn step_for(planned: &PlannedStep) -> Step {
    let mut step = Step::new(
        &planned.name,
        "bash",
        &["-eo", "pipefail", "-c", &planned.run],
        "fix it, then rerun `cargo xtask gate`",
    );
    step.env = planned.env.clone().into_iter().collect();
    step
}

/// HEAD, whether the tree has changes, and each pinned sibling's.
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    /// HEAD's SHA, ` (dirty)` when the tree has changes.
    pub head: String,
    /// Each sibling's HEAD, which equals its pin and is clean.
    pub siblings: Vec<String>,
}

fn state(siblings: &[(String, String)]) -> Result<State, String> {
    let head = checkout(".")?;
    let siblings = siblings
        .iter()
        .map(|(dir, pin_file)| pinned(dir, pin_file))
        .collect::<Result<_, _>>()?;
    Ok(State { head, siblings })
}

/// A sibling checkout, which must be clean and at its pin.
fn pinned(dir: &str, pin_file: &str) -> Result<String, String> {
    let text = std::fs::read_to_string(pin_file).map_err(|e| format!("{pin_file}: {e}"))?;
    let pin = text
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap_or_default()
        .trim()
        .to_owned();
    let head = checkout(dir)?;
    if head != pin {
        return Err(format!(
            "{dir} is at {head}, not {pin} from {pin_file}, or has changes; fix: git -C {dir} checkout --detach {pin} on a clean tree"
        ));
    }
    Ok(head)
}

/// `dir`'s HEAD SHA, ` (dirty)` when it has changes; git must exit 0.
fn checkout(dir: &str) -> Result<String, String> {
    let sha = git(dir, &["rev-parse", "HEAD"])?;
    let changes = git(dir, &["status", "--porcelain"])?;
    Ok(if changes.is_empty() {
        sha
    } else {
        format!("{sha} (dirty)")
    })
}

/// git in `dir`, its stdout trimmed; hook variables stripped (CLAUDE.md).
fn git(dir: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git -C {dir} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
#[path = "gate_tests.rs"]
mod tests;
