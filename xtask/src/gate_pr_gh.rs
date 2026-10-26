//! The gh calls of `gate --pr` (`gate_pr.rs`): finding the PR and
//! posting the comment, each within `GH_DEADLINE`.

use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::gate_pr::{Pr, parse_pr};

/// Each gh call's deadline. Measured 2026-10-26 on the dev box: `gh pr
/// view` took 417 to 508 ms over five calls, `gh pr list` 455 to 556 ms;
/// 30 s is 54 times the slowest, room for a slow GitHub.
pub const GH_DEADLINE: Duration = Duration::from_secs(30);

/// The PR numbered `number`, or this branch's.
pub fn find(number: Option<u64>) -> Result<Pr, String> {
    let mut command = Command::new("gh");
    command.args(["pr", "view"]);
    if let Some(number) = number {
        command.arg(number.to_string());
    }
    command.args(["--json", "number,headRefOid"]);
    parse_pr(&gh(command)?)
}

pub fn post(number: u64, body: &str) -> Result<(), String> {
    let file = std::env::current_dir()
        .map_err(|e| format!("the current directory: {e}"))?
        .join("target/gate/pr_comment.md");
    std::fs::write(&file, body).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut command = Command::new("gh");
    command
        .args(["pr", "comment", &number.to_string(), "--body-file"])
        .arg(&file);
    gh(command).map(|_| ())
}

/// A gh call within `GH_DEADLINE`; its output when it succeeded.
fn gh(command: Command) -> Result<String, String> {
    match run_with_deadline(command, GH_DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => Ok(output),
        Finished::Exited { output, .. } => Err(format!("gh refused: {}", output.trim())),
        Finished::TimedOut { .. } => Err(format!("gh did not answer within {GH_DEADLINE:?}")),
    }
}
