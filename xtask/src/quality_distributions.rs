//! `cargo xtask quality --distributions`: the measurement the thresholds
//! were set from, printed again. File sizes with the lognormal p99
//! behind the file cap, clippy's own counts of every function (a clippy
//! run at threshold 1, `quality_clippy.rs`, joined to xtask's functions,
//! `quality_join.rs`), the field chains, and the agreement between
//! xtask's counts and clippy's. Under the agreement floors the report
//! ends in a refusal, after its tables, so a toolchain change that moves
//! clippy's rules is found the day the command runs
//! (docs/decisions/2026-11-13-quality-thresholds.md).

use crate::quality_clippy::{FLOOR_COMPLEXITY_PER_MILLE, FLOOR_LINES_PER_MILLE};
use crate::quality_collect::{Collected, Split};
use crate::quality_join::{Agreement, Joined};
use crate::quality_stats::{fitted_p99, summary_lines, worst_lines};

/// Lines to print, and whether they end in a refusal.
pub struct Report {
    /// The lines.
    pub lines: Vec<String>,
    /// Set when the command must fail, after its lines.
    pub refusal: Option<String>,
}

/// The tables, the agreement, and the refusal under a floor.
pub fn distribution_report(collected: &Collected, joined: &Joined) -> Report {
    let mut lines = split_lines("file lines", &collected.files);
    for (name, split) in [
        ("code", &collected.files.code),
        ("tests", &collected.files.tests),
    ] {
        let values: Vec<usize> = split.iter().map(|s| s.value).collect();
        lines.push(format!(
            "file lines, {name}: lognormal through median and p90 gives p99 {}",
            fitted_p99(&values)
        ));
    }
    lines.extend(split_lines(
        "function code lines (clippy's too_many_lines, threshold 1)",
        &joined.lines,
    ));
    lines.extend(split_lines(
        "cognitive complexity (clippy's, threshold 1)",
        &joined.complexity,
    ));
    lines.extend(summary_lines(
        "field chain hops, code",
        &collected.chains,
        10,
    ));
    lines.push(agreement_line(
        "lines",
        joined.lines_agreement,
        FLOOR_LINES_PER_MILLE,
    ));
    lines.push(agreement_line(
        "complexity",
        joined.complexity_agreement,
        FLOOR_COMPLEXITY_PER_MILLE,
    ));
    lines.extend(worst_lines(&joined.mismatches, 10));
    Report {
        lines,
        refusal: refusal(joined),
    }
}

fn refusal(joined: &Joined) -> Option<String> {
    let under = joined.lines_agreement.per_mille() < FLOOR_LINES_PER_MILLE
        || joined.complexity_agreement.per_mille() < FLOOR_COMPLEXITY_PER_MILLE;
    under.then(|| {
        "xtask's counts have drifted from clippy's (a toolchain change?); fix: bring \
         quality_complexity.rs or quality_lines.rs back to clippy's rules, or record the \
         new difference there and in the decision note"
            .to_owned()
    })
}

fn agreement_line(what: &str, agreement: Agreement, floor: usize) -> String {
    format!(
        "agreement with clippy on {what}: {}/{} ({}.{}%, floor {}.{}%)",
        agreement.agreed,
        agreement.compared,
        agreement.per_mille() / 10,
        agreement.per_mille() % 10,
        floor / 10,
        floor % 10
    )
}

/// Code and tests, each with its ten worst.
pub fn split_lines(name: &str, split: &Split) -> Vec<String> {
    let mut lines = summary_lines(&format!("{name}, code"), &split.code, 10);
    lines.extend(summary_lines(&format!("{name}, tests"), &split.tests, 10));
    lines
}

#[cfg(test)]
#[path = "quality_distributions_tests.rs"]
mod tests;
