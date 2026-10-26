//! What `check-docs` checks the docs against: xtask's own subcommands
//! (clap's definition, so no help text is parsed), the workspace's
//! examples (files under each package's `examples/`), the app's
//! `--help`, and gh's state of a PR. The two calls that run something
//! have deadlines.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use clap::CommandFactory;

use crate::deadline::{Finished, run_with_deadline};
use crate::docs_plan::Pr;

/// How long the app's `--help` may take, a build of the app included.
/// Chosen until measured: a debug build of touche-app from a warm sccache
/// took about a minute here under load.
const APP_HELP_DEADLINE: Duration = Duration::from_mins(10);

/// xtask's subcommand names, hidden ones included.
#[must_use]
pub fn xtask_commands() -> Vec<String> {
    crate::cli::Cli::command()
        .get_subcommands()
        .map(|c| c.get_name().to_owned())
        .collect()
}

/// Whether `package` (its `[package] name`, a crate under `crates/`) has
/// the example `example` (`examples/<example>.rs` or `<example>/main.rs`).
#[must_use]
pub fn has_example(root: &Path, package: &str, example: &str) -> bool {
    package_dir(root, package).is_some_and(|dir| {
        let examples = dir.join("examples");
        examples.join(format!("{example}.rs")).is_file()
            || examples.join(example).join("main.rs").is_file()
    })
}

fn package_dir(root: &Path, package: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root.join("crates")).ok()?;
    entries.flatten().map(|e| e.path()).find(|dir| {
        std::fs::read_to_string(dir.join("Cargo.toml")).is_ok_and(|toml| {
            toml.lines()
                .any(|l| l.trim() == format!("name = \"{package}\""))
        })
    })
}

/// The app's `cargo run -q -p <package> -- --help`.
///
/// # Errors
///
/// It did not run, failed, or took longer than its deadline.
pub fn app_help(package: &str) -> Result<String, String> {
    let mut command = Command::new("cargo");
    command.args(["run", "-q", "-p", package, "--", "--help"]);
    match run_with_deadline(command, APP_HELP_DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => Ok(output),
        Finished::Exited { output, .. } => Err(format!(
            "`cargo run -p {package} -- --help` failed: {}",
            output.trim()
        )),
        Finished::TimedOut { .. } => Err(format!(
            "`cargo run -p {package} -- --help` took longer than {APP_HELP_DEADLINE:?}"
        )),
    }
}

/// PR `number` of `repo` as gh reports it.
///
/// # Errors
///
/// gh's refusal, no answer in time, or an answer in another shape.
pub fn pr(repo: &str, number: u64) -> Result<Pr, String> {
    let mut command = Command::new("gh");
    command.args([
        "pr",
        "view",
        &number.to_string(),
        "--repo",
        repo,
        "--json",
        "state,isDraft",
    ]);
    let deadline = crate::gate_pr_gh::GH_DEADLINE;
    let json = match run_with_deadline(command, deadline)? {
        Finished::Exited {
            success: true,
            output,
        } => output,
        Finished::Exited { output, .. } => {
            return Err(format!("gh refused #{number}: {}", output.trim()));
        }
        Finished::TimedOut { .. } => {
            return Err(format!(
                "gh did not answer for #{number} within {deadline:?}"
            ));
        }
    };
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("gh's answer for #{number} is not JSON ({e})"))?;
    Ok(Pr {
        state: value["state"]
            .as_str()
            .ok_or_else(|| format!("gh gave #{number} no state"))?
            .to_owned(),
        draft: value["isDraft"]
            .as_bool()
            .ok_or_else(|| format!("gh gave #{number} no isDraft"))?,
    })
}
