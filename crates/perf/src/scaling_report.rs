//! A scaling run as Markdown, and its verdict.

use std::fmt::Write;

use crate::rows::RowChange;
use crate::scaling::{Judged, MIN_GROWTH_INSTRUCTIONS, Scaling, TOLERANCE_PERCENT};

/// The counts of every phase at each size, with the growth per extra
/// row between neighbouring sizes.
#[must_use]
pub fn scaling_section(change: RowChange, scaling: &Scaling) -> String {
    let mut out = format!("### Scaling, {change:?} (median instructions per frame)\n\n| phase |");
    for rows in &scaling.rows {
        let _ = write!(out, " {rows} rows |");
    }
    for pair in scaling.rows.windows(2) {
        let _ = write!(out, " per extra row, {} to {} |", pair[0], pair[1]);
    }
    out.push('\n');
    out.push_str(&"|---".repeat(scaling.rows.len() * 2));
    out.push_str("|\n");
    for series in &scaling.phases {
        let _ = write!(out, "| {} |", series.phase);
        for count in &series.counts {
            let _ = write!(out, " {count} |");
        }
        for at in 1..scaling.rows.len() {
            let _ = write!(out, " {:.1} |", series.per_row(&scaling.rows, at - 1, at));
        }
        out.push('\n');
    }
    out
}

/// The line a passing run ends with.
///
/// # Errors
/// A phase grew beyond the tolerance (each named, with its growth per
/// extra row and the command that reproduces it), or the judged phase
/// has no count.
pub fn verdict(change: RowChange, scaling: &Scaling) -> Result<String, String> {
    let judged = Judged::of(change);
    let violations = scaling.violations(judged)?;
    if !violations.is_empty() {
        return Err(format!(
            "scaling: {change:?} does work that depends on the page's size:\n- {}\nreproduce: \
             vapors-perf 60 10 --scaling {change:?}; the fix is to make the phase's work depend \
             on what changed (docs/plans/perf-finish-line.md, properties 1 and 6)",
            violations.join("\n- ")
        ));
    }
    Ok(match judged {
        Judged::Nothing => format!(
            "scaling: {change:?} is reported, not judged: property 3 (the splice step) is not built"
        ),
        Judged::UpdateTotal => format!(
            "scaling: pass, {change:?} update within {TOLERANCE_PERCENT}% or {MIN_GROWTH_INSTRUCTIONS} instructions of its 1000-row count"
        ),
        Judged::EveryPhase => format!(
            "scaling: pass, every {change:?} phase within {TOLERANCE_PERCENT}% or {MIN_GROWTH_INSTRUCTIONS} instructions of its 1000-row count"
        ),
    })
}

#[cfg(test)]
#[path = "scaling_report_tests.rs"]
mod tests;
