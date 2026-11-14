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
///
/// Relaxed from 9500 (2026-11-13): main measured 9538, 9541.5 and 9542
/// per row, the accessibility mirror's per-row cost since #258 tracks
/// hoisted nodes; 9600 is that rounded up to the next hundred, ten
/// times the repeat noise. Layout-lane PR 5 (the accessibility mirror
/// in flow, `docs/plans/splice-layout.md`) retires it, restoring 9500.
pub const INSERT_A11Y_PER_ROW: u64 = 9600;

/// As [`INSERT_A11Y_PER_ROW`] for `FlexInsert`, on the flex-column list,
/// whose rows are one block holding their text: measured 1829.6 and
/// 1832.9 per row (insert and remove frames, 2026-11-14, layout-lane PR
/// 3f-2); 1900 is that rounded up to the next hundred. Layout-lane PR 5
/// (the accessibility mirror in flow) retires it.
pub const FLEX_INSERT_A11Y_PER_ROW: u64 = 1900;

/// As [`INSERT_A11Y_PER_ROW`] for `Resize`: recorded at 12,021.
///
/// Relaxed from 13,000 (2026-11-13) for the same reason: main measured
/// 13,443 and 13,446.3 per row. Layout-lane PR 5 retires it, restoring
/// 13,000.
pub const RESIZE_A11Y_PER_ROW: u64 = 13_500;

/// As [`INSERT_A11Y_PER_ROW`] for `Scroll`: the phase is flat (6789 at
/// every size: the take reads the scrolled container alone, its rows'
/// bounds being from it), so one instruction per extra row, 9000 at
/// 10,000 rows, is less than one more node read.
pub const SCROLL_A11Y_PER_ROW: u64 = 1;

/// As [`SCROLL_A11Y_PER_ROW`] for `ScrollSize`: the body is laid out
/// again in the frame it scrolls, and the rows the edit did not move
/// stay scrolled, so the phase reads the middle row's nodes and the
/// body alone. Recorded in `docs/perf/a11y-phase-2026-11-12.md`,
/// "After PR 5a".
pub const SCROLL_SIZE_A11Y_PER_ROW: u64 = 1;

/// The tolerance for `FlexText`'s `frame (CPU)`, in percent, in place of
/// [`crate::scaling::TOLERANCE_PERCENT`].
///
/// Relaxed from 2% (2026-11-13, lead decision): a one-time step from
/// 1,000 to 3,000 rows, then flat; about 60% is glibc malloc on a larger
/// heap (2.0-2.3% under glibc, 0.9% under jemalloc), the remaining
/// ~2.9k instructions unidentified. Retired when the residual's source
/// is found and the allocator part is removed (style-allocations S1 or
/// an allocator decision), then 2% again.
pub const FLEX_TEXT_FRAME_PERCENT: u64 = 3;

/// Which phases a change is judged on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Judged {
    /// Every phase: an edit's work should not depend on size.
    EveryPhase,
    /// [`Judged::EveryPhase`], with `phase` held to `percent` in place of
    /// the tolerance: a declared relaxation, its reason at its constant.
    EveryPhaseRelaxed {
        /// The phase whose tolerance is relaxed.
        phase: &'static str,
        /// Its tolerance, in percent.
        percent: u64,
    },
    /// The accessibility phase alone, held to `ceiling` instructions
    /// per extra row: the change moves every row below it, and the
    /// mirror's take reads each moved row (`docs/plans/r6.md`, "Known
    /// non-constant costs"), and `flat` held as `EveryPhase` holds every
    /// phase. The other phases are reported, not judged: property 3 of
    /// `docs/plans/perf-finish-line.md` is not built.
    AccessibilityPerRow {
        /// The most instructions per extra row the phase may cost.
        ceiling: u64,
        /// Phases whose cost may not grow with the rows.
        flat: &'static [&'static str],
    },
}

impl Judged {
    /// What `change` is judged on.
    #[must_use]
    pub fn of(change: RowChange) -> Self {
        match change {
            RowChange::FlexText => Self::EveryPhaseRelaxed {
                phase: "frame (CPU)",
                percent: FLEX_TEXT_FRAME_PERCENT,
            },
            RowChange::Idle | RowChange::Text | RowChange::Class | RowChange::HoveredClass => {
                Self::EveryPhase
            }
            // style flat since docs/perf/insert-style-2026-11-13.md
            RowChange::Insert => Self::AccessibilityPerRow {
                ceiling: INSERT_A11Y_PER_ROW,
                flat: &["style"],
            },
            RowChange::FlexInsert => Self::AccessibilityPerRow {
                ceiling: FLEX_INSERT_A11Y_PER_ROW,
                flat: &["style"],
            },
            RowChange::Resize => Self::AccessibilityPerRow {
                ceiling: RESIZE_A11Y_PER_ROW,
                flat: &[],
            },
            RowChange::Scroll => Self::AccessibilityPerRow {
                ceiling: SCROLL_A11Y_PER_ROW,
                flat: &[],
            },
            RowChange::ScrollSize => Self::AccessibilityPerRow {
                ceiling: SCROLL_SIZE_A11Y_PER_ROW,
                flat: &[],
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
