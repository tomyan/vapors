//! The gh calls of `gate --pr` (`gate_pr.rs`): finding the PR and
//! posting the comment, each within `GH_DEADLINE`. Every call keeps to
//! `--json`: `gh pr view N` without it exits 1 here on a GraphQL
//! deprecation notice.

use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::gate_pr::{Pr, parse_pr};

/// Each gh call's deadline. Measured 2026-10-26 on the dev box: `gh pr
/// view` took 417 to 508 ms over five calls; `gh pr comment` is not
/// measured and is given the same. 30 s is 59 times the slowest view.
pub const GH_DEADLINE: Duration = Duration::from_secs(30);

const FIELDS: &str = "number,headRefOid,state";

/// The PR numbered `number`, or this branch's.
pub fn find(number: Option<u64>) -> Result<Pr, String> {
    let mut command = Command::new("gh");
    command.args(["pr", "view"]);
    if let Some(number) = number {
        command.arg(number.to_string());
    }
    command.args(["--json", FIELDS]);
    parse_pr(&gh(command)?)
}

/// PR `number` of `repo` (`owner/name`), from anywhere (the queue).
pub fn find_in(repo: &str, number: u64) -> Result<Pr, String> {
    let mut command = Command::new("gh");
    command.args([
        "pr",
        "view",
        &number.to_string(),
        "--repo",
        repo,
        "--json",
        FIELDS,
    ]);
    parse_pr(&gh(command)?)
}

/// Post `body` to PR `number`, kept first in `target/gate/pr_comment.md`
/// (made here: a gate refused before its own preflight leaves none).
pub fn post(number: u64, body: &str) -> Result<(), String> {
    let dir = std::env::current_dir()
        .map_err(|e| format!("the current directory: {e}"))?
        .join("target/gate");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join("pr_comment.md");
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
