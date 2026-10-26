//! A work item's State cell, as `docs_plan.rs` reads it. The cell starts
//! with a plain state (`planned`, … with any note after `, `, `; ` or
//! ` (`), `merged `, or a PR (`[<repo> ][draft ]PR #N`). Every `#N` in
//! the cell is then a claim, its state given by the words before it:
//! `PR #N` open (a draft when `draft` goes before or `(draft` after),
//! `draft #N` a draft, and a number in the list after `merged ` (up to a
//! `(` or `;`) merged. A repository may be named first (`vapors PR #N`,
//! `merged vapors #N`). A `#N` anywhere else is refused by name, since its
//! state could not be checked.

use crate::docs_plan::{Expect, PLAIN};

/// A PR a State names: its repository (none for `[docs] repo`), number
/// and claimed state.
pub type Named = (Option<String>, u64, Expect);

/// Why a State cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unread {
    /// It starts in no shape this reads.
    Shape,
    /// It names this PR where its state cannot be read.
    Token(String),
}

/// Every PR `state` names, with the state its words give it.
///
/// # Errors
///
/// The State's shape is unknown, or a PR in it cannot be classified.
pub fn state_claims(state: &str) -> Result<Vec<Named>, Unread> {
    let names_prs = state.starts_with("merged ") || pr_start(state);
    if !names_prs && !plain_start(state) {
        return Err(Unread::Shape);
    }
    let list_end = state
        .strip_prefix("merged ")
        .map(|_| state.find(['(', ';']).unwrap_or(state.len()));
    let claims = references(state)
        .into_iter()
        .map(|(at, len, pr)| classify(state, (at, len, pr), list_end))
        .collect::<Result<Vec<_>, _>>()?;
    if names_prs && claims.is_empty() {
        return Err(Unread::Shape);
    }
    Ok(claims)
}

fn plain_start(state: &str) -> bool {
    PLAIN.iter().any(|plain| {
        state.strip_prefix(plain).is_some_and(|rest| {
            rest.is_empty() || [", ", "; ", " ("].iter().any(|sep| rest.starts_with(sep))
        })
    })
}

/// Whether the State opens with `[<repo> ][draft ]PR #N`.
fn pr_start(state: &str) -> bool {
    let Some((at, ..)) = references(state).first().copied() else {
        return false;
    };
    let before: Vec<&str> = state[..at].split_whitespace().collect();
    match before[..] {
        ["PR"] | ["draft", "PR"] => true,
        [repo, "PR"] | [repo, "draft", "PR"] => is_repo(repo),
        _ => false,
    }
}

/// Each `#N` in `state`: where its `#` is, how many digits, the number.
fn references(state: &str) -> Vec<(usize, usize, u64)> {
    state
        .match_indices('#')
        .filter_map(|(at, _)| {
            let digits: String = state[at + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            Some((at, digits.len(), digits.parse().ok()?))
        })
        .collect()
}

/// The claim the words around one `#N` make.
fn classify(
    state: &str,
    (at, len, pr): (usize, usize, u64),
    list_end: Option<usize>,
) -> Result<Named, Unread> {
    let before: Vec<&str> = state[..at].split_whitespace().collect();
    let back = |n: usize| before.len().checked_sub(n + 1).map(|i| before[i]);
    let after = state[at + 1 + len..].trim_start();
    match back(0) {
        Some("PR") => {
            let drafted = back(1) == Some("draft");
            let repo = back(if drafted { 2 } else { 1 });
            let expect = if drafted || after.starts_with("(draft") {
                Expect::Draft
            } else {
                Expect::Open
            };
            Ok((repo_named(repo), pr, expect))
        }
        Some("draft") => Ok((repo_named(back(1)), pr, Expect::Draft)),
        _ if list_end.is_some_and(|end| at < end) => Ok((repo_named(back(0)), pr, Expect::Merged)),
        _ => Err(Unread::Token(format!("#{pr}"))),
    }
}

fn repo_named(word: Option<&str>) -> Option<String> {
    word.filter(|w| is_repo(w)).map(str::to_owned)
}

/// A repository's name: a lowercase word that is none of the grammar's.
fn is_repo(word: &str) -> bool {
    !["draft", "merged"].contains(&word)
        && !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
