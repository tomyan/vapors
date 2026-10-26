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
    /// Whether it comes from a fork (gh's isCrossRepository).
    pub fork: bool,
    /// The first file it changes under xtask/, .github/ or
    /// .config/factory.toml, which only the lead may clear.
    pub protected: Option<String>,
    /// Whether its clear is signed (lead).
    pub cleared_by_lead: bool,
    /// Whether its head's last gate posted nothing within the backoff.
    pub backed_off: bool,
    /// Whether it merges on GitHub's checks, not a gate comment.
    pub checks: bool,
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
    let number = pr.number;
    if let Some(why) = waiting(pr) {
        return Decision::Skip(format!("#{number} {why}"));
    }
    match (pr.contains_main, pr.gate) {
        (true, Gate::Passed) => Decision::Merge,
        (true, Gate::Failed) => Decision::Skip(format!("#{number}'s gate failed at {}", pr.head)),
        (true, Gate::None) if pr.checks => {
            Decision::Skip(format!("#{number}'s checks are running"))
        }
        (true, Gate::None) if pr.backed_off => Decision::Skip(format!(
            "#{number}'s last gate at {} posted nothing; it is retried after its backoff",
            pr.head
        )),
        (false, _) if !pr.rebase_clean => {
            Decision::Skip(format!("#{number} conflicts with main on rebase"))
        }
        _ => Decision::Regate,
    }
}

/// Why the PR is not the queue's to act on yet, if it is not.
fn waiting(pr: &Facts) -> Option<String> {
    let said = if pr.held {
        "is held (queue: hold)"
    } else if pr.draft {
        "is a draft"
    } else if pr.fork {
        "is from a fork; the queue pushes only to this repository"
    } else if !pr.base_is_main {
        "is based on another branch, not main"
    } else {
        match pr.clear {
            Clear::Missing => "has no review clear",
            Clear::Changed => "changed since review clear",
            Clear::Holds => return protected(pr),
        }
    };
    Some(said.to_owned())
}

/// A factory change cleared by anyone but the lead.
fn protected(pr: &Facts) -> Option<String> {
    let path = pr.protected.as_ref().filter(|_| !pr.cleared_by_lead)?;
    Some(format!(
        "touches {path}, so it needs a clear signed (lead), not (foreman)"
    ))
}

#[cfg(test)]
#[path = "queue_decide_tests.rs"]
mod tests;
