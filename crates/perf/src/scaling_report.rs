//! A scaling run as Markdown, and its verdict.

use std::fmt::Write;

use crate::rows::RowChange;
use crate::scaling::{Judged, MIN_GROWTH_INSTRUCTIONS, PhaseSeries, Scaling, TOLERANCE_PERCENT};

/// The counts of every phase at each size, with the growth per extra
/// row between neighbouring sizes.
#[must_use]
pub fn scaling_section(change: RowChange, scaling: &Scaling) -> String {
    let mut out = header(change, scaling);
    for series in &scaling.phases {
        out.push_str(&phase_row(series, &scaling.rows));
    }
    out
}

fn header(change: RowChange, scaling: &Scaling) -> String {
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
    out
}

fn phase_row(series: &PhaseSeries, rows: &[usize]) -> String {
    let mut out = format!("| {} |", series.phase);
    for count in &series.counts {
        let _ = write!(out, " {count} |");
    }
    for at in 1..rows.len() {
        let _ = write!(out, " {:.1} |", series.per_row(rows, at - 1, at));
    }
    out.push('\n');
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
        Judged::AccessibilityPerRow { ceiling, flat } => format!(
            "scaling: pass, {change:?} accessibility within {ceiling} instructions per extra row{}; \
             the other phases are reported, not judged: property 3 (the splice step) is not built",
            held_flat(flat)
        ),
        Judged::EveryPhase => format!(
            "scaling: pass, every {change:?} phase within {TOLERANCE_PERCENT}% or {MIN_GROWTH_INSTRUCTIONS} instructions of its 1000-row count"
        ),
    })
}

/// The phases a moving change holds flat, for its pass line.
fn held_flat(flat: &[&str]) -> String {
    if flat.is_empty() {
        return String::new();
    }
    format!(" and {} flat", flat.join(", "))
}

#[cfg(test)]
#[path = "scaling_report_tests.rs"]
mod tests;
