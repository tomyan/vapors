//! What a PR's comments say to the merge queue: held (a comment whose
//! last line is `queue: hold`), the newest review clear (last line
//! `review: clear <sha> (lead)` or `(foreman)`), and the gate of the
//! current head: the newest `gate --pr` comment for it, read only through
//! `gate_pr::parse_header`, the one parser of those comments (`gate --pr`
//! posts exit 0 only for a gate that passed that head).
//!
//! The roles are a convention, not a security boundary: every comment
//! comes from the same GitHub account, and an author could sign one.
//! Declared limits: a hold is lifted only by editing or deleting its
//! comment; reviews and review comments are not read, so a clear posted
//! as a review is not seen.

use crate::gate_pr::{Ended, parse_header};
use crate::queue_decide::Gate;

/// The last line with text in it.
fn last_line(comment: &str) -> &str {
    comment
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}

/// Who may clear a PR, as the clear's last word.
const CLEARED_BY: [&str; 2] = ["(lead)", "(foreman)"];

/// Whether a comment holds the PR.
#[must_use]
pub fn held(comments: &[String]) -> bool {
    comments.iter().any(|c| last_line(c) == "queue: hold")
}

/// A review clear: the commit it cleared, and whether the lead signed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleared {
    /// The cleared commit's full SHA.
    pub sha: String,
    /// Signed (lead), not (foreman).
    pub by_lead: bool,
}

/// The newest review clear, if any: a full 40-hex SHA and a known signer.
#[must_use]
pub fn cleared(comments: &[String]) -> Option<Cleared> {
    comments.iter().rev().find_map(|c| {
        let rest = last_line(c).strip_prefix("review: clear ")?;
        let (sha, who) = rest.split_once(' ')?;
        let full = sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit());
        (full && CLEARED_BY.contains(&who)).then(|| Cleared {
            sha: sha.to_owned(),
            by_lead: who == "(lead)",
        })
    })
}

/// The gate of `head`, from the newest gate comment for it.
#[must_use]
pub fn gate(comments: &[String], head: &str) -> Gate {
    let for_head = comments
        .iter()
        .rev()
        .filter_map(|c| parse_header(c))
        .find(|(sha, _)| sha == head);
    match for_head {
        None => Gate::None,
        Some((_, Ended::Code(0))) => Gate::Passed,
        Some(_) => Gate::Failed,
    }
}

#[cfg(test)]
#[path = "queue_comments_tests.rs"]
mod tests;
