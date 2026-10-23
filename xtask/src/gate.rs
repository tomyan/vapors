//! `cargo xtask gate`: every `run` step `.github/workflows/ci.yml` runs,
//! local composite actions' included, from this worktree, as CI runs it:
//! in file order, each script by `bash -eo pipefail -c` with the
//! environment CI gives it (`gate_plan`), each job within its
//! `timeout-minutes` (`gate_run`), stopping at the first failure. The
//! runner's setup is left out by exact line (`[gate.setup]`), and
//! third-party actions by name (`[gate.actions]`), each with its reason.
//! It runs only on a clean tree and ends with `gate: pass <HEAD sha>`;
//! HEAD, the tree and each pinned sibling (`[gate.pinned]`: a checkout
//! that must be clean at the commit its pin file names) are read before
//! and after, and any change, or an unreadable state, fails the gate.
//! Locally every job runs in this one worktree, one after another; CI
//! gives each job a fresh runner.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use crate::config::Config;
use crate::gate_plan::{PlannedStep, Sources, plan};
use crate::report::Reporter;

/// How a state with changes in its tree is marked.
const DIRTY: &str = " (dirty)";

/// The workflow the gate runs.
pub const WORKFLOW: &str = ".github/workflows/ci.yml";

/// Run the gate.
pub fn run(reporter: &Reporter) -> ExitCode {
    let prepared = Config::load()
        .map_err(|e| e.to_string())
        .and_then(|config| prepare(&config));
    let (steps, before, siblings, output) = match prepared {
        Ok(prepared) => prepared,
        Err(why) => return refuse(&why),
    };
    let code = crate::gate_run::run(&steps, &output, reporter);
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

/// The plan, the state before (a clean tree), the pinned siblings, and
/// the `GITHUB_OUTPUT` file, emptied.
fn prepare(config: &Config) -> Result<(Vec<PlannedStep>, State, Pinned, PathBuf), String> {
    let siblings: Pinned = config.entries_under("gate.pinned.").into_iter().collect();
    let before = state(&siblings)?;
    refuse_dirty(&before)?;
    let workflow = std::fs::read_to_string(WORKFLOW)
        .map_err(|e| format!("{WORKFLOW}: {e}; run from the workspace root"))?;
    let (setup, actions) = (setup_lines(config), actions(config)?);
    let read = |path: &str| std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"));
    let sources = Sources {
        setup: &setup,
        actions: &actions,
        read: &read,
    };
    let planned = plan(&workflow, &sources).map_err(|why| format!("{WORKFLOW}: {why}"))?;
    Ok((planned, before, siblings, output_file()?))
}

/// The gate passes only a commit: a tree with changes is refused.
fn refuse_dirty(before: &State) -> Result<(), String> {
    if before.head.ends_with(DIRTY) {
        return Err(format!(
            "the tree has changes ({}); the gate passes only a commit; fix: commit or stash, then rerun",
            before.head
        ));
    }
    Ok(())
}

/// `target/gate/github_output`, emptied, as an absolute path: composite
/// steps run from elsewhere.
fn output_file() -> Result<PathBuf, String> {
    let dir = std::env::current_dir()
        .map_err(|e| format!("the current directory: {e}"))?
        .join("target/gate");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join("github_output");
    std::fs::write(&file, "").map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(file)
}

/// `[gate.actions]`: third-party actions the gate does not run, each
/// with its reason, which must not be empty.
pub fn actions(config: &Config) -> Result<BTreeMap<String, String>, String> {
    let actions = config.entries_under("gate.actions.");
    match actions.iter().find(|(_, reason)| reason.trim().is_empty()) {
        Some((name, _)) => Err(format!(
            "[gate.actions] `{name}` has no reason; fix: say why it need not run locally"
        )),
        None => Ok(actions),
    }
}

/// `[gate.setup]` keys: the runner's setup, exact lines.
#[must_use]
pub fn setup_lines(config: &Config) -> BTreeSet<String> {
    config.entries_under("gate.setup.").into_keys().collect()
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
        format!("{sha}{DIRTY}")
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
