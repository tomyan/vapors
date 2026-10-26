//! The merge queue's calls that leave the machine: gh (listing open PRs
//! with their comments, merging, retargeting, deleting a branch) and git
//! fetch and push. Each runs within its deadline, and its refusal is
//! returned as text for the queue's log.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::gate_pr_gh::GH_DEADLINE;

/// git fetch and push's deadline. Measured 2026-10-26 on the dev box: a
/// fetch of main took 769 to 899 ms over four; 60 s is 67 times that.
pub const GIT_REMOTE_DEADLINE: Duration = Duration::from_secs(60);

/// One open PR as gh lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// Its number.
    pub number: u64,
    /// Its head commit.
    pub head: String,
    /// Its head branch.
    pub branch: String,
    /// The branch it targets.
    pub base: String,
    /// Whether it is a draft.
    pub draft: bool,
    /// Its comments' bodies, oldest first.
    pub comments: Vec<String>,
}

/// gh's `pr list --json` answer, oldest PR first.
///
/// # Errors
///
/// Not JSON, or a PR missing a field, naming both.
pub fn parse_list(json: &str) -> Result<Vec<Listed>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("gh's PR list is not JSON ({e})"))?;
    let mut prs = value
        .as_array()
        .ok_or("gh's PR list is not a list")?
        .iter()
        .map(listed)
        .collect::<Result<Vec<_>, _>>()?;
    prs.sort_by_key(|pr| pr.number);
    Ok(prs)
}

fn listed(pr: &serde_json::Value) -> Result<Listed, String> {
    let number = pr["number"].as_u64().ok_or("a listed PR has no number")?;
    let text = |field: &str| {
        pr[field]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("#{number} has no {field} in gh's list"))
    };
    let comments = pr["comments"]
        .as_array()
        .ok_or_else(|| format!("#{number} has no comments in gh's list"))?;
    Ok(Listed {
        number,
        head: text("headRefOid")?,
        branch: text("headRefName")?,
        base: text("baseRefName")?,
        draft: pr["isDraft"]
            .as_bool()
            .ok_or_else(|| format!("#{number} has no isDraft in gh's list"))?,
        comments: comments
            .iter()
            .filter_map(|c| c["body"].as_str().map(str::to_owned))
            .collect(),
    })
}

/// The open PRs of `repo` (`owner/name`), oldest first.
///
/// # Errors
///
/// gh's refusal, or an answer in another shape.
pub fn open_prs(repo: &str) -> Result<Vec<Listed>, String> {
    let fields = "number,headRefOid,headRefName,baseRefName,isDraft,comments";
    parse_list(&gh(&[
        "pr", "list", "--repo", repo, "--state", "open", "--limit", "100", "--json", fields,
    ])?)
}

/// Squash-merge PR `number` of `repo`, only while its head is `head`.
///
/// # Errors
///
/// gh's refusal (a head that moved, a conflict), as text.
pub fn merge(repo: &str, number: u64, head: &str) -> Result<(), String> {
    gh(&[
        "pr",
        "merge",
        &number.to_string(),
        "--repo",
        repo,
        "--squash",
        "--match-head-commit",
        head,
    ])
    .map(|_| ())
}

/// Point PR `number` at main (the REST API: gh's GraphQL edit fails on
/// the projects-classic deprecation).
///
/// # Errors
///
/// gh's refusal.
pub fn retarget(repo: &str, number: u64) -> Result<(), String> {
    gh(&[
        "api",
        "-X",
        "PATCH",
        &format!("repos/{repo}/pulls/{number}"),
        "-f",
        "base=main",
    ])
    .map(|_| ())
}

/// Delete `branch` on GitHub.
///
/// # Errors
///
/// gh's refusal.
pub fn delete_branch(repo: &str, branch: &str) -> Result<(), String> {
    gh(&[
        "api",
        "-X",
        "DELETE",
        &format!("repos/{repo}/git/refs/heads/{branch}"),
    ])
    .map(|_| ())
}

/// `git fetch origin <refs>` in `dir`.
///
/// # Errors
///
/// git's refusal, or no answer within the deadline.
pub fn fetch(dir: &Path, refs: &[&str]) -> Result<(), String> {
    let mut args = vec!["fetch", "-q", "origin"];
    args.extend_from_slice(refs);
    git_remote(dir, &args)
}

/// Push `new` to `branch`, only if GitHub still has it at `known`.
///
/// # Errors
///
/// git's refusal (the branch moved: the lease failed), or no answer.
pub fn push_lease(dir: &Path, branch: &str, known: &str, new: &str) -> Result<(), String> {
    let lease = format!("--force-with-lease={branch}:{known}");
    let to = format!("{new}:refs/heads/{branch}");
    git_remote(dir, &["push", "-q", &lease, "origin", &to])
}

fn gh(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("gh");
    command.args(args);
    finished(run_with_deadline(command, GH_DEADLINE)?, "gh", GH_DEADLINE)
}

fn git_remote(dir: &Path, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(dir).args(args);
    for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        command.env_remove(name);
    }
    finished(
        run_with_deadline(command, GIT_REMOTE_DEADLINE)?,
        "git",
        GIT_REMOTE_DEADLINE,
    )
    .map(|_| ())
}

fn finished(finished: Finished, what: &str, deadline: Duration) -> Result<String, String> {
    match finished {
        Finished::Exited {
            success: true,
            output,
        } => Ok(output),
        Finished::Exited { output, .. } => Err(format!("{what} refused: {}", output.trim())),
        Finished::TimedOut { .. } => Err(format!("{what} did not answer within {deadline:?}")),
    }
}

#[cfg(test)]
#[path = "queue_remote_tests.rs"]
mod tests;
