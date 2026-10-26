//! The PLAN.md rule of `cargo xtask check-docs`: each work item's State
//! names its PRs as gh sees them. Only tables headed `| Id | What | Owner
//! | State | Depends on |` are read. A State is, by the grammar at the
//! top of docs/PLAN.md:
//! - `planned`, `in progress`, `parked`, `pending Tom`, `in review fixes`:
//!   no PR claimed; a note may follow after `, `, `; ` or ` (`, and
//!   must name no PR;
//! - `merged #N[, #M…]`, optionally followed by a note: each PR merged;
//! - `PR #N …`: open and not a draft (`PR #N (draft) …`: an open draft);
//! - `draft PR #N …`: an open draft.
//!
//! A PR may name its repository first (`vapors #N`, `vapors PR #N`), a
//! repository under `[docs] repo`'s owner; without one it is `[docs]
//! repo`'s.
//!
//! Any other State is refused, naming its line.

/// The work-item table's header.
pub const HEADER: &str = "| Id | What | Owner | State | Depends on |";

/// States that claim no PR.
const PLAIN: [&str; 5] = [
    "planned",
    "in progress",
    "parked",
    "pending Tom",
    "in review fixes",
];

/// What a State says a PR is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Merged.
    Merged,
    /// Open, not a draft.
    Open,
    /// Open, a draft.
    Draft,
}

/// One PR a State names, on its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// The line in the file.
    pub line: usize,
    /// The repository it names (`vapors`), under `[docs] repo`'s owner;
    /// none for `[docs] repo` itself.
    pub repo: Option<String>,
    /// The PR's number.
    pub pr: u64,
    /// What the State says it is.
    pub expect: Expect,
}

/// A PR as gh reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pr {
    /// `OPEN`, `CLOSED` or `MERGED`.
    pub state: String,
    /// Whether it is a draft.
    pub draft: bool,
}

/// Every claim in `text`'s work-item tables, and each State in no shape.
#[must_use]
pub fn claims(file: &str, text: &str) -> (Vec<Claim>, Vec<String>) {
    let (mut claims, mut problems) = (Vec::new(), Vec::new());
    let mut in_table = false;
    for (n, line) in text.lines().enumerate() {
        let line_no = n + 1;
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        if line.trim() == HEADER {
            in_table = true;
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if !in_table || cells.get(1).is_some_and(|c| c.starts_with("---")) {
            continue;
        }
        let state = cells.get(4).copied().unwrap_or_default();
        match state_claims(state) {
            Some(found) => claims.extend(found.into_iter().map(|(repo, pr, expect)| Claim { line: line_no, repo, pr, expect })),
            None => problems.push(format!(
                "{file}:{line_no}: the State \"{state}\" is in no shape this check reads; fix: write it as the grammar at the top of {file} says"
            )),
        }
    }
    (claims, problems)
}

/// A PR a State names: its repository, number and claimed state.
type Named = (Option<String>, u64, Expect);

/// The PRs a State claims, or `None` for a State in no shape.
fn state_claims(state: &str) -> Option<Vec<Named>> {
    if plain(state) {
        return Some(Vec::new());
    }
    if let Some(rest) = state.strip_prefix("merged ") {
        let listed = rest.split('(').next().unwrap_or(rest);
        return listed
            .split(',')
            .map(|part| reference(part.trim()).map(|(repo, pr)| (repo, pr, Expect::Merged)))
            .collect();
    }
    pr_state(state).map(|named| vec![named])
}

/// A plain State, alone or with a note after `, `, `; ` or ` (` that
/// names no PR (a PR in a note would go unchecked).
fn plain(state: &str) -> bool {
    let noted = |plain: &&str| {
        state.strip_prefix(*plain).is_some_and(|note| {
            note.is_empty() || [", ", "; ", " ("].iter().any(|sep| note.starts_with(sep))
        })
    };
    PLAIN.iter().any(noted) && !names_pr(state)
}

fn names_pr(text: &str) -> bool {
    text.match_indices('#')
        .any(|(at, _)| text[at + 1..].starts_with(|c: char| c.is_ascii_digit()))
}

/// `[<repo> ]PR #N …` or `[<repo> ]draft PR #N …`.
fn pr_state(state: &str) -> Option<Named> {
    let (repo, rest) = repo_prefix(state, |rest| {
        rest.starts_with("PR #") || rest.starts_with("draft PR #")
    });
    let (rest, draft) = match rest.strip_prefix("draft PR #") {
        Some(rest) => (rest, true),
        None => (rest.strip_prefix("PR #")?, false),
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let after = rest[digits.len()..].trim_start();
    let expect = if draft || after.starts_with("(draft") {
        Expect::Draft
    } else {
        Expect::Open
    };
    Some((repo, digits.parse().ok()?, expect))
}

/// `[<repo> ]#N`.
fn reference(part: &str) -> Option<(Option<String>, u64)> {
    let (repo, rest) = repo_prefix(part, |rest| rest.starts_with('#'));
    Some((repo, rest.strip_prefix('#')?.parse().ok()?))
}

/// A leading repository name, a lowercase word other than `draft`,
/// before what `follows` accepts; the rest of `text`.
fn repo_prefix(text: &str, follows: impl Fn(&str) -> bool) -> (Option<String>, &str) {
    let is_name = |word: &str| {
        word != "draft"
            && !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    match text.split_once(' ') {
        Some((word, rest)) if is_name(word) && follows(rest) => (Some(word.to_owned()), rest),
        _ => (None, text),
    }
}

/// How a claim's PR is named in a message: `#N`, or `<repo> #N`.
#[must_use]
pub fn label(claim: &Claim) -> String {
    match &claim.repo {
        Some(repo) => format!("{repo} #{}", claim.pr),
        None => format!("#{}", claim.pr),
    }
}

/// What is wrong with `claim`, given what gh says of its PR.
#[must_use]
pub fn contradiction(file: &str, claim: &Claim, pr: &Pr) -> Option<String> {
    let listed = match claim.expect {
        Expect::Merged => "merged",
        Expect::Open => "open",
        Expect::Draft => "a draft",
    };
    let holds = match claim.expect {
        Expect::Merged => pr.state == "MERGED",
        Expect::Open => pr.state == "OPEN" && !pr.draft,
        Expect::Draft => pr.state == "OPEN" && pr.draft,
    };
    let actual = match (pr.state.as_str(), pr.draft) {
        ("OPEN", true) => "an open draft".to_owned(),
        (state, _) => state.to_ascii_lowercase(),
    };
    (!holds).then(|| {
        format!(
            "{file}:{}: {} is listed as {listed}, but gh says it is {actual}; fix: update its State",
            claim.line,
            label(claim)
        )
    })
}

#[cfg(test)]
#[path = "docs_plan_tests.rs"]
mod tests;
