//! What one pass of the merge queue (`queue.rs`) does with one open
//! pull request, from what is known about it: merge it, gate it (after
//! rebasing it onto main when that is clean), or skip it with the reason
//! logged for the foreman. A skip that needs work (a conflict, a failed
//! gate, a missing or outdated review clear) is left for an agent.

/// Whether the PR's review clear (`review: clear <sha> (<who>)`) still
/// describes it: the same patch-id set as the cleared commit had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clear {
    /// No clear posted.
    Missing,
    /// Cleared, and every commit since is the same patch.
    Holds,
    /// Cleared, but a commit changed since (a fix, a conflict resolution).
    Changed,
}

/// The gate's comment for the PR's current head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// None posted for this head.
    None,
    /// Posted, ending `gate: pass <head>`.
    Passed,
    /// Posted, and the gate failed.
    Failed,
}

/// What is known about one open PR.
// Allowed: each bool is one independent fact about the PR that `decide`
// reads; folding them into enums would name the same yes-or-no twice.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// Its number.
    pub number: u64,
    /// Its head commit.
    pub head: String,
    /// Whether it targets main.
    pub base_is_main: bool,
    /// Whether it is a draft.
    pub draft: bool,
    /// Whether a comment ends `queue: hold`.
    pub held: bool,
    /// Its review clear.
    pub clear: Clear,
    /// Whether its head contains origin/main.
    pub contains_main: bool,
    /// Its gate for the current head.
    pub gate: Gate,
    /// Whether `git rebase origin/main` is conflict-free (asked only when
    /// it is behind).
    pub rebase_clean: bool,
}

/// What the pass does with a PR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Squash-merge it at its head.
    Merge,
    /// Rebase it onto main when behind, push, and gate it.
    Regate,
    /// Leave it; the reason is logged.
    Skip(String),
}

/// The decision for `pr`.
#[must_use]
pub fn decide(pr: &Facts) -> Decision {
    if let Some(why) = waiting(pr) {
        return Decision::Skip(format!("#{} {why}", pr.number));
    }
    match (pr.contains_main, pr.gate) {
        (true, Gate::Passed) => Decision::Merge,
        (true, Gate::Failed) => {
            Decision::Skip(format!("#{}'s gate failed at {}", pr.number, pr.head))
        }
        (false, _) if !pr.rebase_clean => {
            Decision::Skip(format!("#{} conflicts with main on rebase", pr.number))
        }
        _ => Decision::Regate,
    }
}

/// Why the PR is not the queue's to act on yet, if it is not.
fn waiting(pr: &Facts) -> Option<&'static str> {
    if pr.held {
        return Some("is held (queue: hold)");
    }
    if pr.draft {
        return Some("is a draft");
    }
    if !pr.base_is_main {
        return Some("is based on another branch, not main");
    }
    match pr.clear {
        Clear::Missing => Some("has no review clear"),
        Clear::Changed => Some("changed since review clear"),
        Clear::Holds => None,
    }
}

#[cfg(test)]
#[path = "queue_decide_tests.rs"]
mod tests;
