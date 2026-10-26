//! `cargo xtask gate --pr [N]`: the gate, and its last lines posted to
//! the pull request it gated, so no person or agent copies them by hand
//! and a merge queue can act on the comment alone. The PR is N, or the
//! one whose head is this branch. Before anything runs, its head must be
//! this checkout's HEAD (the gate itself refuses a dirty tree). The gate
//! runs as a child (`xtask gate`), its output passed through to the
//! terminal and kept. After it, the PR's head is read again: if it moved
//! during the run, nothing is posted, and that is said. The exit code is
//! the gate's; a post that fails is a failure.

use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};

use crate::gate_pr_gh::{find, post};

/// How many of the gate's last lines the comment carries.
pub const TAIL_LINES: usize = 30;

/// A pull request: its number and its head commit.
#[derive(Debug, PartialEq, Eq)]
pub struct Pr {
    /// The PR's number.
    pub number: u64,
    /// Its head commit's full SHA.
    pub head: String,
}

/// Run the gate for `pr` (or this branch's PR) and post its tail.
pub fn run(pr: Option<u64>) -> ExitCode {
    let pr = match find(pr).and_then(|pr| same_head(&pr, &local_head()?).map(|()| pr)) {
        Ok(pr) => pr,
        Err(why) => return refuse(&why),
    };
    let (code, output) = match gate_child() {
        Ok(ran) => ran,
        Err(why) => return refuse(&why),
    };
    match find(Some(pr.number)).and_then(|now| still_gated(&pr, &now.head)) {
        Ok(()) => posted(&pr, code, &output),
        Err(why) => refuse(&format!("not posted: {why}")),
    }
}

/// The gate's exit code when the comment was posted, else a failure.
fn posted(pr: &Pr, code: i32, output: &str) -> ExitCode {
    let body = comment(&pr.head, pr.number, code, &tail(output, TAIL_LINES));
    if let Err(why) = post(pr.number, &body) {
        return refuse(&format!(
            "the gate's output was not posted to #{}: {why}",
            pr.number
        ));
    }
    println!("gate --pr: posted to #{}", pr.number);
    if code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn refuse(why: &str) -> ExitCode {
    println!("FAIL gate --pr: {why}");
    ExitCode::FAILURE
}

/// The last `lines` lines of `output`, joined by newlines.
#[must_use]
pub fn tail(output: &str, lines: usize) -> String {
    let all: Vec<&str> = output.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// The comment: the head, the command and the exit code, then the tail.
#[must_use]
pub fn comment(head: &str, pr: u64, code: i32, tail: &str) -> String {
    format!(
        "Gate output for {head} (`cargo xtask gate --pr {pr}`, exit {code}):\n\n```\n{tail}\n```\n"
    )
}

/// gh's `--json number,headRefOid` for one PR.
///
/// # Errors
///
/// Not JSON, or a field missing, naming it.
pub fn parse_pr(json: &str) -> Result<Pr, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("gh's answer is not JSON ({e}): {json}"))?;
    let number = value["number"]
        .as_u64()
        .ok_or_else(|| format!("gh's answer has no number: {json}"))?;
    let head = value["headRefOid"]
        .as_str()
        .ok_or_else(|| format!("gh's answer has no headRefOid: {json}"))?;
    Ok(Pr {
        number,
        head: head.to_owned(),
    })
}

/// The PR's head is this checkout's HEAD.
///
/// # Errors
///
/// Both SHAs, and the fix.
pub fn same_head(pr: &Pr, local: &str) -> Result<(), String> {
    if pr.head == local {
        return Ok(());
    }
    Err(format!(
        "#{}'s head is {}, but HEAD is {local}; fix: push this branch, or check out the PR's head, then rerun",
        pr.number, pr.head
    ))
}

/// The PR's head is still the commit the gate ran on.
///
/// # Errors
///
/// When it moved, naming the new head.
pub fn still_gated(gated: &Pr, now: &str) -> Result<(), String> {
    if gated.head == now {
        return Ok(());
    }
    Err(format!(
        "#{}'s head moved to {now} during the gate on {}; fix: rerun the gate on the new head",
        gated.number, gated.head
    ))
}

/// `xtask gate` as a child: its exit code, and its output, which also
/// goes to the terminal as it comes. The gate bounds itself: each job
/// runs within its `timeout-minutes` (`gate_run`).
fn gate_child() -> Result<(i32, String), String> {
    let exe = std::env::current_exe().map_err(|e| format!("this xtask's path: {e}"))?;
    let mut child = Command::new(exe)
        .arg("gate")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start the gate: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("the gate's output could not be read")?;
    let mut output = String::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|e| format!("reading the gate's output: {e}"))?;
        println!("{line}");
        output.push_str(&line);
        output.push('\n');
    }
    let status = child
        .wait()
        .map_err(|e| format!("waiting for the gate: {e}"))?;
    Ok((status.code().unwrap_or(1), output))
}

/// This checkout's HEAD; git must exit 0 (hook variables stripped).
fn local_head() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse HEAD: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
#[path = "gate_pr_tests.rs"]
mod tests;
