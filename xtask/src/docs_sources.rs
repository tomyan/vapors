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
use crate::docs_patterns::Source;
use crate::docs_plan::Pr;

/// How long the app's `--help` may take, a build of the app included.
/// Measured 2026-10-26: a cold build of touche-app and its `--help`, with
/// an empty target directory, no sccache, nice 10 and 4 jobs at load 16,
/// took 104 s; warm, 0.2 s. 8 min is about 4.6 times the cold build and
/// stays below the ci job's 9 min timeout, so this deadline fires first.
const APP_HELP_DEADLINE: Duration = Duration::from_mins(8);

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

/// The extensions of the files the page's text can be in.
const PAGE_EXTENSIONS: [&str; 6] = ["rs", "html", "js", "mjs", "css", "json"];

/// The page's source files under `dirs` (space-separated, relative to
/// `root`), outside build output (`target`, `node_modules`) and hidden
/// directories; each path as written relative to `root`.
///
/// # Errors
///
/// A directory or file that cannot be read, naming it.
pub fn page_sources(root: &Path, dirs: &str) -> Result<Vec<Source>, String> {
    let mut sources = Vec::new();
    for dir in dirs.split_whitespace() {
        collect(root, Path::new(dir), &mut sources)
            .map_err(|e| format!("[docs] page: {dir}: {e}"))?;
    }
    Ok(sources)
}

fn collect(root: &Path, rel: &Path, sources: &mut Vec<Source>) -> Result<(), String> {
    let named = |e: std::io::Error| format!("{}: {e}", rel.display());
    for entry in std::fs::read_dir(root.join(rel)).map_err(named)? {
        let entry = entry.map_err(named)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = rel.join(&name);
        if entry.file_type().map_err(named)?.is_dir() {
            if !(name.starts_with('.') || name == "target" || name == "node_modules") {
                collect(root, &path, sources)?;
            }
        } else if is_page_text(&path) {
            let text = std::fs::read_to_string(root.join(&path))
                .map_err(|e| format!("{}: {e}", path.display()))?;
            sources.push(Source::new(&path.to_string_lossy(), &text));
        }
    }
    Ok(())
}

fn is_page_text(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| PAGE_EXTENSIONS.contains(&e))
}

#[cfg(test)]
#[path = "docs_sources_tests.rs"]
mod tests;
