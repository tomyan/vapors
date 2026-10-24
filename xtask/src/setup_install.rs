//! Installing one pinned tool with `cargo binstall`: download only, by a
//! deadline, and a failure that quotes binstall's own error.

use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::tools::Tool;

/// Each pinned tool downloaded in 2.7 to 3.4 s on the dev machine
/// (2026-10-23). A rate-limited GitHub API makes binstall wait 120 s
/// and retry (vapors run 40029521532), which this cuts short.
const DEADLINE: Duration = Duration::from_secs(60);

/// Lines of binstall's output quoted in a failure.
const QUOTED_LINES: usize = 10;

/// How a binstall run ended, with its output.
pub enum Ended {
    Failed(String),
    TimedOut(String),
}

/// The line printed before installing an absent or mismatched tool.
pub fn before_install(tool: &Tool, found: Option<&str>) -> String {
    let name = &tool.crate_name;
    match found {
        None => format!(
            "missing {name:<18} {} not installed; installing",
            tool.version
        ),
        Some(found) => format!(
            "wrong   {name:<18} found {found}, pinned {}; installing",
            tool.version
        ),
    }
}

/// Install `tool` at its pinned version, or say why not.
pub fn install(tool: &Tool) -> Result<(), String> {
    let spec = format!("{}@{}", tool.crate_name, tool.version);
    // Not through sccache: setup installs sccache, so it cannot need it.
    let mut command = Command::new("cargo");
    command.args(binstall_args(&spec));
    let ended = match run_with_deadline(command, DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => {
            print!("{output}");
            return Ok(());
        }
        Finished::Exited { output, .. } => Ended::Failed(output),
        Finished::TimedOut { output } => Ended::TimedOut(output),
    };
    Err(failure(&spec, &ended))
}

/// Compiling is disabled: a download failure must fail here, by name,
/// not turn into a minutes-long source build (vapors run 40029521532).
pub fn binstall_args(spec: &str) -> Vec<String> {
    ["binstall", "-y", "--disable-strategies", "compile", spec]
        .map(String::from)
        .to_vec()
}

/// The failure message: what failed, binstall's last lines, the
/// reproducing command.
pub fn failure(spec: &str, ended: &Ended) -> String {
    let (what, output) = match ended {
        Ended::Failed(output) => ("binstall failed".to_owned(), output),
        Ended::TimedOut(output) => (
            format!("binstall did not finish within {} s", DEADLINE.as_secs()),
            output,
        ),
    };
    format!(
        "could not download {spec}: {what}; its last output:\n{}\n\
         fix: on CI, check GITHUB_TOKEN reaches the setup step (binstall uses it \
         for the GitHub API); reproduce: cargo {}",
        last_lines(output),
        binstall_args(spec).join(" ")
    )
}

fn last_lines(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(QUOTED_LINES);
    lines[start..].join("\n")
}

#[cfg(test)]
#[path = "setup_install_tests.rs"]
mod tests;
