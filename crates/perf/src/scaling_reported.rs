//! Changes `--scaling` reports without judging (`Judged::Reported`):
//! each names its owner, a slice of a plan that a test checks exists and
//! names the change, and the rule it will be judged by. A reported
//! change that passes that rule fails the run, so the owner's pull
//! request cannot land without turning the judging on.

use crate::rows::RowChange;
use crate::scaling::Scaling;
use crate::scaling_judged::Judged;
use crate::scaling_report::verdict_by;

/// A slice of a plan, by the plan's path from the repository root and
/// the slice's id in its slices table (`| 3b | ...`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owner {
    /// The plan, from the repository root.
    pub plan: &'static str,
    /// The slice's id, the first cell of its row.
    pub slice: &'static str,
}

impl std::fmt::Display for Owner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} slice {}", self.plan, self.slice)
    }
}

/// Layout-lane slice 3b: a lane container's absolute and fixed children
/// placed from the tree.
pub const SPLICE_LAYOUT_3B: Owner = Owner {
    plan: "docs/plans/splice-layout.md",
    slice: "3b",
};

/// Splice slice D: a box with a stacking of its own inserted or freed
/// spliced into its context's layer in place of the whole walk.
pub const SPLICE_D: Owner = Owner {
    plan: "docs/plans/splice.md",
    slice: "D stacked splice",
};

/// The verdict lines of reported `change`, one per kind: reported, not
/// judged, while some kind still fails `target`.
///
/// # Errors
/// Every kind passes `target`: the change is to be judged by it now.
pub fn reported_verdicts(
    change: RowChange,
    (owner, target): (Owner, Judged),
    kinds: &[(String, Scaling)],
) -> Result<String, String> {
    let passes =
        |(title, scaling): &(String, Scaling)| verdict_by(target, change, title, scaling).is_ok();
    if kinds.iter().all(passes) {
        return Err(format!(
            "scaling: {change:?} now passes the rule it was reported for ({target:?}): judge it by \
             that rule in Judged::of (crates/perf/src/scaling_judged.rs), as {owner} says; \
             reproduce: vapors-perf 60 10 --scaling {change:?}"
        ));
    }
    let lines: Vec<String> = kinds
        .iter()
        .map(|(title, _)| {
            format!("scaling: reported, not judged, {title}: {owner} judges it as {target:?}")
        })
        .collect();
    Ok(lines.join("\n"))
}

#[cfg(test)]
#[path = "scaling_reported_tests.rs"]
mod tests;
