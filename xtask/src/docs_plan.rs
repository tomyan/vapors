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

use crate::docs_plan_state::{Unread, state_claims};

/// The work-item table's header.
pub const HEADER: &str = "| Id | What | Owner | State | Depends on |";

/// States that claim no PR.
pub const PLAIN: [&str; 5] = [
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
#[cfg(test)]
#[must_use]
pub fn claims(file: &str, text: &str) -> (Vec<Claim>, Vec<String>) {
    let (claims, problems, _) = read(file, text);
    (claims, problems)
}

/// [`claims`], and a problem when the plan compares nothing: no
/// work-item table, or tables that name no PR.
#[must_use]
pub fn read_plan(file: &str, text: &str) -> (Vec<Claim>, Vec<String>) {
    let (claims, mut problems, seen_table) = read(file, text);
    let no_claims = claims.is_empty() && problems.is_empty();
    problems.extend(nothing_read(file, seen_table, no_claims));
    (claims, problems)
}

/// The claims, the problems, and whether a work-item table was seen.
fn read(file: &str, text: &str) -> (Vec<Claim>, Vec<String>, bool) {
    let (mut claims, mut problems) = (Vec::new(), Vec::new());
    let (mut in_table, mut seen_table) = (false, false);
    for (n, line) in text.lines().enumerate() {
        let line_no = n + 1;
        if !line.trim_start().starts_with('|') {
            in_table = false;
            continue;
        }
        if let Some(header) = header(file, line_no, line, &mut problems) {
            (in_table, seen_table) = (header, seen_table || header);
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if !in_table || cells.get(1).is_some_and(|c| c.starts_with("---")) {
            continue;
        }
        let state = cells.get(4).copied().unwrap_or_default();
        match state_claims(state) {
            Ok(found) => claims.extend(found.into_iter().map(|(repo, pr, expect)| Claim {
                line: line_no,
                repo,
                pr,
                expect,
            })),
            Err(unread) => problems.push(unread_problem(file, line_no, state, &unread)),
        }
    }
    (claims, problems, seen_table)
}

/// For a table's header line: whether it opens a work-item table. A
/// header with a State column but another shape is refused.
/// Column names that mark a work-item table, however its header differs
/// (a State column renamed Status would otherwise go unread).
const WORK_ITEM_COLUMNS: [&str; 5] = ["Id", "Owner", "State", "Status", "Depends on"];

fn header(file: &str, line_no: usize, line: &str, problems: &mut Vec<String>) -> Option<bool> {
    if line.trim() == HEADER {
        return Some(true);
    }
    let work_item = line
        .split('|')
        .map(str::trim)
        .any(|cell| WORK_ITEM_COLUMNS.contains(&cell));
    if !work_item {
        return None;
    }
    problems.push(format!(
        "{file}:{line_no}: a work-item table is headed `{}`, not `{HEADER}`; fix: use that header, or rename the columns so it is not a work-item table",
        line.trim()
    ));
    Some(false)
}

fn unread_problem(file: &str, line_no: usize, state: &str, unread: &Unread) -> String {
    match unread {
        Unread::Shape => format!(
            "{file}:{line_no}: the State \"{state}\" is in no shape this check reads; fix: write it as the grammar at the top of {file} says"
        ),
        Unread::Token(pr) => format!(
            "{file}:{line_no}: the State \"{state}\" names {pr} where this check cannot read its state; fix: write it as PR {pr}, draft PR {pr} or merged {pr}"
        ),
    }
}

/// A plan with no work-item table, or none naming a PR, compares nothing.
fn nothing_read(file: &str, seen_table: bool, no_claims: bool) -> Option<String> {
    if !seen_table {
        return Some(format!(
            "{file}: no table is headed `{HEADER}`; fix: keep the work items in such a table"
        ));
    }
    no_claims.then(|| format!(
        "{file}: its work-item tables name no PR, so nothing is compared with gh; fix: check that the States are read (the grammar at the top of the file)"
    ))
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

#[cfg(test)]
#[path = "docs_plan_grammar_tests.rs"]
mod grammar_tests;
