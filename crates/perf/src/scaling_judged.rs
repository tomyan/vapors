//! Which phases `--scaling` judges for each change, and the judge of a
//! phase whose cost is allowed to grow with the rows a change moves:
//! the accessibility phase of Insert, Resize and Scroll, held to a
//! ceiling of instructions per extra row recorded in
//! `docs/perf/a11y-phase-2026-11-12.md` (review-v242 M2).

use crate::rows::RowChange;
use crate::scaling::PhaseSeries;

/// The accessibility phase's ceiling for `Insert`, instructions per
/// extra row from 1000 rows: recorded at 8859 (1000 to 10,000 rows,
/// `docs/perf/a11y-phase-2026-11-12.md`, "After PR 5a"), with 7% of
/// room over the 0.06% repeat noise. It holds until the splice step
/// bounds what an insert moves (property 3 of
/// `docs/plans/perf-finish-line.md`).
pub const INSERT_A11Y_PER_ROW: u64 = 9500;

/// As [`INSERT_A11Y_PER_ROW`] for `Resize`: recorded at 12,021.
pub const RESIZE_A11Y_PER_ROW: u64 = 13_000;

/// As [`INSERT_A11Y_PER_ROW`] for `Scroll`: the phase is flat (6789 at
/// every size: the take reads the scrolled container alone, its rows'
/// bounds being from it), so one instruction per extra row, 9000 at
/// 10,000 rows, is less than one more node read.
pub const SCROLL_A11Y_PER_ROW: u64 = 1;

/// Which phases a change is judged on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judged {
    /// Every phase: an edit's work should not depend on size.
    EveryPhase,
    /// The accessibility phase alone, held to `ceiling` instructions
    /// per extra row: the change moves every row below it, and the
    /// mirror's take reads each moved row (`docs/plans/r6.md`, "Known
    /// non-constant costs"). The other phases are reported, not judged:
    /// property 3 of `docs/plans/perf-finish-line.md` is not built.
    AccessibilityPerRow {
        /// The most instructions per extra row the phase may cost.
        ceiling: u64,
    },
}

impl Judged {
    /// What `change` is judged on.
    #[must_use]
    pub fn of(change: RowChange) -> Self {
        match change {
            RowChange::Idle
            | RowChange::Text
            | RowChange::FlexText
            | RowChange::Class
            | RowChange::HoveredClass => Self::EveryPhase,
            RowChange::Insert => Self::AccessibilityPerRow {
                ceiling: INSERT_A11Y_PER_ROW,
            },
            RowChange::Resize => Self::AccessibilityPerRow {
                ceiling: RESIZE_A11Y_PER_ROW,
            },
            RowChange::Scroll => Self::AccessibilityPerRow {
                ceiling: SCROLL_A11Y_PER_ROW,
            },
        }
    }
}

/// The sizes at which `series` costs more than `ceiling` instructions
/// per extra row over the first size, one message each.
pub(crate) fn beyond_ceiling(series: &PhaseSeries, rows: &[usize], ceiling: u64) -> Vec<String> {
    (1..rows.len())
        .map(|at| (at, series.per_row(rows, 0, at)))
        .filter(|&(_, per_row)| per_row > ceiling_as_f64(ceiling))
        .map(|(at, per_row)| over_ceiling(series, rows, at, per_row, ceiling))
        .collect()
}

#[allow(clippy::cast_precision_loss)] // ceilings under 2^52
fn ceiling_as_f64(ceiling: u64) -> f64 {
    ceiling as f64
}

fn over_ceiling(
    series: &PhaseSeries,
    rows: &[usize],
    at: usize,
    per_row: f64,
    ceiling: u64,
) -> String {
    format!(
        "{} costs {per_row:.1} instructions per extra row from {} to {} rows ({} to {} \
         instructions), over its ceiling of {ceiling} (docs/perf/a11y-phase-2026-11-12.md: the \
         take reads each row the change moved, and the ceiling holds until the splice step bounds \
         what moves)",
        series.phase, rows[0], rows[at], series.counts[0], series.counts[at]
    )
}
