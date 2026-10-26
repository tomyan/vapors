//! What a PR's comments say to the merge queue: held (a comment whose
//! last line is `queue: hold`), the newest review clear (last line
//! `review: clear <sha> (<who>)`), and the gate of the current head (the
//! newest `Gate output for <head>` comment, from `gate --pr`, passing only
//! with a line `gate: pass <head>`).

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

/// Whether a comment holds the PR.
#[must_use]
pub fn held(comments: &[String]) -> bool {
    comments.iter().any(|c| last_line(c) == "queue: hold")
}

/// The sha of the newest review clear, if any.
#[must_use]
pub fn cleared(comments: &[String]) -> Option<String> {
    comments.iter().rev().find_map(|c| {
        let rest = last_line(c).strip_prefix("review: clear ")?;
        let (sha, who) = rest.split_once(' ')?;
        let named = who.starts_with('(') && who.ends_with(')') && who.len() > 2;
        named.then(|| sha.to_owned())
    })
}

/// The gate of `head`, from the newest gate comment for it.
#[must_use]
pub fn gate(comments: &[String], head: &str) -> Gate {
    let opening = format!("Gate output for {head} ");
    let pass = format!("gate: pass {head}");
    match comments.iter().rev().find(|c| c.starts_with(&opening)) {
        None => Gate::None,
        Some(c) if c.lines().any(|l| l.trim() == pass) => Gate::Passed,
        Some(_) => Gate::Failed,
    }
}

#[cfg(test)]
#[path = "queue_comments_tests.rs"]
mod tests;
