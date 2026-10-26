//! `cargo xtask gate --pr [N]`: the gate, and its last lines posted to
//! the pull request it gated, so no person or agent copies them by hand
//! and the merge queue acts on the comment alone. `run_with` holds the
//! sequence over its effects (`Effects`; the real ones are in
//! `gate_pr_real.rs`):
//! 1. find the PR (N, or this branch's), which must be open, with its
//!    head at this checkout's HEAD;
//! 2. run the gate;
//! 3. on exit 0, require its last line to be `gate: pass <the PR's head>`
//!    (the gate reads HEAD only once it holds the lock, and HEAD can move
//!    while it waits);
//! 4. read the PR's head again, and post nothing if it moved;
//! 5. refuse a body that holds a secret's value;
//! 6. post it.
//!
//! The comment's first line, read back by `parse_header` (the queue's
//! one parser), names the head and how the gate ended.

use crate::gate_pr_text::not_posted;
pub use crate::gate_pr_text::{TAIL_LINES, comment, parse_header, tail};

/// A pull request: its number, its head commit, and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr {
    /// The PR's number.
    pub number: u64,
    /// Its head commit's full SHA.
    pub head: String,
    /// `OPEN`, `CLOSED` or `MERGED`.
    pub state: String,
}

/// How the gate ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    /// It exited with this code.
    Code(i32),
    /// A signal killed it (the OOM killer, a stop).
    Signal(i32),
}

impl Ended {
    /// `exit 0`, or `killed by signal 9`: the comment's words.
    #[must_use]
    pub fn words(self) -> String {
        match self {
            Self::Code(code) => format!("exit {code}"),
            Self::Signal(signal) => format!("killed by signal {signal}"),
        }
    }
}

/// The gate's ending and its output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    /// How it ended.
    pub ended: Ended,
    /// What it printed.
    pub output: String,
}

/// What `run_with` does outside itself.
pub trait Effects {
    /// PR `number`, or this branch's.
    fn find(&self, number: Option<u64>) -> Result<Pr, String>;
    /// This checkout's HEAD.
    fn local_head(&self) -> Result<String, String>;
    /// Run the gate.
    fn gate(&self) -> Result<Ran, String>;
    /// Post `body` to PR `number`.
    fn post(&self, number: u64, body: &str) -> Result<(), String>;
    /// The environment's secret-looking variables: (name, value).
    fn secrets(&self) -> Vec<(String, String)>;
}

/// Gate PR `number` (or this branch's) and post its tail; how the gate
/// ended, once posted.
///
/// # Errors
///
/// Each refusal of the sequence, said; nothing is posted after one.
pub fn run_with(effects: &impl Effects, number: Option<u64>) -> Result<Ended, String> {
    let pr = effects.find(number)?;
    open(&pr)?;
    same_head(&pr, &effects.local_head()?)?;
    let ran = effects.gate()?;
    if ran.ended == Ended::Code(0) {
        gated_head(&ran.output, &pr)?;
    }
    still_gated(&pr, &effects.find(Some(pr.number))?.head)?;
    let body = comment(
        &pr.head,
        pr.number,
        ran.ended,
        &tail(&ran.output, TAIL_LINES),
    );
    reads_back(&body, &pr, ran.ended)?;
    no_secrets(&body, &effects.secrets())?;
    effects
        .post(pr.number, &body)
        .map_err(|why| not_posted(&pr, ran.ended, &why))?;
    Ok(ran.ended)
}

fn open(pr: &Pr) -> Result<(), String> {
    if pr.state == "OPEN" {
        return Ok(());
    }
    Err(format!(
        "#{} is {}, not open; fix: gate an open PR",
        pr.number, pr.state
    ))
}

/// A passing gate's last line names the PR's head.
fn gated_head(output: &str, pr: &Pr) -> Result<(), String> {
    let last = output
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let gated = last.strip_prefix("gate: pass ").unwrap_or(last);
    if gated == pr.head {
        return Ok(());
    }
    Err(format!(
        "the gate passed {gated}, not #{}'s head {}; nothing posted; fix: keep HEAD at the PR's head while the gate waits for its lock, then rerun",
        pr.number, pr.head
    ))
}

/// The comment's first line reads back, through the queue's parser, as
/// the head and ending it was written with.
fn reads_back(body: &str, pr: &Pr, ended: Ended) -> Result<(), String> {
    if parse_header(body) == Some((pr.head.clone(), ended)) {
        return Ok(());
    }
    Err(format!(
        "the comment's first line does not read back as #{}'s head and the gate's ending; this is a bug in gate_pr_text.rs",
        pr.number
    ))
}

fn no_secrets(body: &str, secrets: &[(String, String)]) -> Result<(), String> {
    match secrets
        .iter()
        .find(|(_, value)| value.len() >= 8 && body.contains(value.as_str()))
    {
        Some((name, _)) => Err(format!(
            "the gate's output holds the value of {name}; nothing posted; fix: find the step that printed it"
        )),
        None => Ok(()),
    }
}

/// gh's `--json number,headRefOid,state` for one PR.
///
/// # Errors
///
/// Not JSON, or a field missing, naming it.
pub fn parse_pr(json: &str) -> Result<Pr, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("gh's answer is not JSON ({e}): {json}"))?;
    let field = |name: &str| {
        value[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("gh's answer has no {name}: {json}"))
    };
    let number = value["number"]
        .as_u64()
        .ok_or_else(|| format!("gh's answer has no number: {json}"))?;
    Ok(Pr {
        number,
        head: field("headRefOid")?,
        state: field("state")?,
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
        "#{}'s head moved to {now} during the gate on {}; nothing posted; fix: rerun the gate on the new head",
        gated.number, gated.head
    ))
}

#[cfg(test)]
#[path = "gate_pr_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gate_pr_run_tests.rs"]
mod run_tests;
