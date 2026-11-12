//! GitHub's checks as a vapors PR's gate (`merge_on = "checks"`), and
//! the branch update that reruns them on main.

use std::process::Command;

use crate::deadline::{Finished, run_with_deadline};
use crate::queue_decide::Gate;

/// GitHub's checks (`gh pr view --json statusCheckRollup`) as a gate:
/// failed when one failed, none yet while one runs, else passed. The
/// gh on this box (2.46) has no `--json` for `pr checks`.
///
/// # Errors
///
/// Not JSON, no checks, or a state this does not know, naming it.
pub fn checks_gate(json: &str) -> Result<Gate, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("gh's checks are not JSON ({e})"))?;
    let checks = value["statusCheckRollup"]
        .as_array()
        .ok_or("gh's answer has no statusCheckRollup")?;
    if checks.is_empty() {
        return Err("the PR has no checks; fix: see that CI ran on it".into());
    }
    let states = checks.iter().map(state).collect::<Result<Vec<_>, _>>()?;
    Ok(if states.contains(&Gate::Failed) {
        Gate::Failed
    } else if states.contains(&Gate::None) {
        Gate::None
    } else {
        Gate::Passed
    })
}

/// One check run's or status context's state.
fn state(check: &serde_json::Value) -> Result<Gate, String> {
    let field = |name: &str| check[name].as_str().unwrap_or_default();
    let (status, conclusion, context) = (field("status"), field("conclusion"), field("state"));
    match (status, conclusion, context) {
        ("COMPLETED", "SUCCESS" | "SKIPPED" | "NEUTRAL", _) | ("", "", "SUCCESS") => {
            Ok(Gate::Passed)
        }
        (
            "COMPLETED",
            "FAILURE" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE",
            _,
        )
        | ("", "", "FAILURE" | "ERROR") => Ok(Gate::Failed),
        ("QUEUED" | "IN_PROGRESS" | "PENDING" | "WAITING" | "REQUESTED", _, _)
        | ("", "", "PENDING" | "EXPECTED") => Ok(Gate::None),
        _ => Err(format!(
            "a check in a state this does not know (status {status:?}, conclusion {conclusion:?}, state {context:?}); fix: add it to queue_checks.rs"
        )),
    }
}

/// PR `number`'s checks as a gate.
///
/// # Errors
///
/// gh's refusal, or an answer in another shape.
pub fn checks(repo: &str, number: u64) -> Result<Gate, String> {
    let mut command = Command::new("gh");
    command.args([
        "pr",
        "view",
        &number.to_string(),
        "--repo",
        repo,
        "--json",
        "statusCheckRollup",
    ]);
    match run_with_deadline(command, crate::gate_pr_gh::GH_DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => checks_gate(&output),
        Finished::Exited { output, .. } => Err(format!("gh refused the checks: {}", output.trim())),
        Finished::TimedOut { .. } => Err("gh did not list the checks in time".into()),
    }
}

/// Ask GitHub to merge main into PR `number`'s branch, so its checks
/// rerun on main.
///
/// # Errors
///
/// gh's refusal, or no answer.
pub fn update_branch(repo: &str, number: u64) -> Result<(), String> {
    let mut command = Command::new("gh");
    command.args(["pr", "update-branch", &number.to_string(), "--repo", repo]);
    match run_with_deadline(command, crate::gate_pr_gh::GH_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "gh refused to update the branch: {}",
            output.trim()
        )),
        Finished::TimedOut { .. } => Err("gh did not answer the branch update".into()),
    }
}

#[cfg(test)]
#[path = "queue_checks_tests.rs"]
mod tests;
