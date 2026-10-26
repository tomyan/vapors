//! The words of `gate --pr`'s comment: the tail it carries, its first
//! line (written by `comment`, read back by `parse_header`, the queue's
//! one parser of gate comments), and the refusal of a post that failed.

use crate::gate_pr::{Ended, Pr};

/// How many of the gate's last lines the comment carries.
pub const TAIL_LINES: usize = 30;

/// A post that failed: what the gate did, why, and the command that
/// posts the kept body.
pub fn not_posted(pr: &Pr, ended: Ended, why: &str) -> String {
    let number = pr.number;
    format!(
        "the gate {}; the comment was not posted to #{number}: {why}; fix: gh pr comment {number} --body-file target/gate/pr_comment.md",
        match ended {
            Ended::Code(code) => format!("exited {code}"),
            Ended::Signal(signal) => format!("was killed by signal {signal}"),
        }
    )
}

/// The last `lines` lines of `output`, joined by newlines.
#[must_use]
pub fn tail(output: &str, lines: usize) -> String {
    let all: Vec<&str> = output.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// The comment: the head, the command and how the gate ended, then the
/// tail in a four-backtick fence (a tail line of three cannot close it).
#[must_use]
pub fn comment(head: &str, pr: u64, ended: Ended, tail: &str) -> String {
    format!(
        "Gate output for {head} (`cargo xtask gate --pr {pr}`, {}):\n\n````\n{tail}\n````\n",
        ended.words()
    )
}

/// A gate comment's head and ending, from its first line; `None` for a
/// comment that is not one. The queue reads gates only through this.
#[must_use]
pub fn parse_header(body: &str) -> Option<(String, Ended)> {
    let line = body.lines().next()?;
    let rest = line.strip_prefix("Gate output for ")?;
    let (head, rest) = rest.split_once(' ')?;
    let words = rest.strip_suffix("):")?.rsplit_once(", ")?.1;
    let ended = if let Some(code) = words.strip_prefix("exit ") {
        Ended::Code(code.parse().ok()?)
    } else {
        Ended::Signal(words.strip_prefix("killed by signal ")?.parse().ok()?)
    };
    Some((head.to_owned(), ended))
}
