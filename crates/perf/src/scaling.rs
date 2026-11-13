//! Whether a change's instruction counts grow with the page: the
//! counts of each phase at several row counts, the growth per extra
//! row, and the phases that grew beyond the tolerance. All pure; the
//! counter and the workloads are in `scaling_run.rs`.

use std::collections::BTreeMap;

pub use crate::scaling_judged::Judged;
use crate::scaling_judged::beyond_ceiling;

/// The row counts a change is measured at, ascending.
pub const SIZES: [usize; 3] = [1000, 3000, 10_000];

/// How far above its 1000-row count a judged phase may be at a larger
/// size, in percent. Counts of one workload repeat within 0.06% on this
/// machine (docs/perf/baseline-2026-11-10.md, "Instruction counts"), so
/// 2% is over thirty times the noise. It is also small against a per-row
/// cost: 2% of a 1,000,000-instruction phase over 9000 extra rows is
/// about 2 instructions per extra row.
pub const TOLERANCE_PERCENT: u64 = 2;

/// The least growth over the 1000-row count that fails a phase, in
/// instructions, so a small phase is not failed by 2% of itself. A 2%
/// step on a phase under 250,000 instructions is under 5000, and one
/// extra instruction per row over 9000 extra rows is 9000: 5000 sits
/// below that, so a phase that gains an instruction per row at 10,000
/// rows still fails. Growth fails only when it exceeds both this and
/// [`TOLERANCE_PERCENT`].
pub const MIN_GROWTH_INSTRUCTIONS: u64 = 5000;

/// Median instructions per phase, at one row count.
#[derive(Debug)]
pub struct AtSize {
    /// The page's row count.
    pub rows: usize,
    /// Median instructions by phase name.
    pub phases: BTreeMap<String, u64>,
}

/// One phase's counts, a count per size of its [`Scaling`].
pub struct PhaseSeries {
    /// The phase's name.
    pub phase: String,
    /// Median instructions at each of the [`Scaling`]'s row counts.
    pub counts: Vec<u64>,
}

/// The counts of every phase across sizes.
pub struct Scaling {
    /// The row counts, ascending.
    pub rows: Vec<usize>,
    /// Each phase's counts.
    pub phases: Vec<PhaseSeries>,
}

impl Scaling {
    /// The series of `measured`, which must be two or more sizes in
    /// ascending order with the same phases.
    ///
    /// # Errors
    /// Fewer than two sizes, sizes not ascending, or a phase counted at
    /// one size and not another (naming the phase and the size).
    pub fn of(measured: &[AtSize]) -> Result<Self, String> {
        let [first, rest @ ..] = measured else {
            return Err("scaling: no sizes were measured".into());
        };
        if rest.is_empty() || measured.windows(2).any(|w| w[0].rows >= w[1].rows) {
            return Err("scaling: needs two or more ascending sizes".into());
        }
        if let Some(extra) = rest.iter().find_map(|size| extra_phase(first, size)) {
            return Err(extra);
        }
        let phases = first.phases.keys().map(|phase| series(measured, phase));
        Ok(Self {
            rows: measured.iter().map(|size| size.rows).collect(),
            phases: phases.collect::<Result<_, _>>()?,
        })
    }

    /// The phases of `judged` that grew beyond [`TOLERANCE_PERCENT`],
    /// or the accessibility phase beyond its ceiling per extra row and
    /// the phases held flat beside it, one message each naming the
    /// phase, the size and the growth per extra row.
    ///
    /// # Errors
    /// `judged` names a phase the series lack (never a pass).
    pub fn violations(&self, judged: Judged) -> Result<Vec<String>, String> {
        match judged {
            Judged::AccessibilityPerRow { ceiling, flat } => {
                let mut found = beyond_ceiling(self.series("accessibility")?, &self.rows, ceiling);
                for phase in flat {
                    found.extend(self.beyond_tolerance(self.series(phase)?, TOLERANCE_PERCENT));
                }
                Ok(found)
            }
            Judged::EveryPhase => Ok(self.beyond_tolerances(|_| TOLERANCE_PERCENT)),
            Judged::EveryPhaseRelaxed { phase, percent } => Ok(self.beyond_tolerances(|named| {
                if named == phase {
                    percent
                } else {
                    TOLERANCE_PERCENT
                }
            })),
        }
    }

    fn series(&self, phase: &str) -> Result<&PhaseSeries, String> {
        self.phases
            .iter()
            .find(|series| series.phase == phase)
            .ok_or_else(|| format!("scaling: no count for phase {phase}, so it cannot be judged"))
    }

    /// Every phase's violations, each phase held to `percent_of(phase)`.
    fn beyond_tolerances(&self, percent_of: impl Fn(&str) -> u64) -> Vec<String> {
        self.phases
            .iter()
            .flat_map(|series| self.beyond_tolerance(series, percent_of(&series.phase)))
            .collect()
    }

    fn beyond_tolerance<'a>(
        &'a self,
        series: &'a PhaseSeries,
        percent: u64,
    ) -> impl Iterator<Item = String> + 'a {
        (1..self.rows.len())
            .filter(move |&at| grew_beyond_both(series.counts[0], series.counts[at], percent))
            .map(move |at| self.violation(series, at, percent))
    }

    fn violation(&self, series: &PhaseSeries, at: usize, percent: u64) -> String {
        format!(
            "{} grows with the page: {} instructions at {} rows, {} at {} rows, \
             {:.1} instructions per extra row (over {percent}% and \
             {MIN_GROWTH_INSTRUCTIONS} instructions above the first size)",
            series.phase,
            series.counts[0],
            self.rows[0],
            series.counts[at],
            self.rows[at],
            series.per_row(&self.rows, 0, at)
        )
    }
}

impl PhaseSeries {
    /// Instructions added per extra row between sizes `from` and `to`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counts under 2^52
    pub fn per_row(&self, rows: &[usize], from: usize, to: usize) -> f64 {
        let added = self.counts[to] as f64 - self.counts[from] as f64;
        added / (rows[to] - rows[from]) as f64
    }
}

/// `count` is above `first` by more than `percent` and by more than
/// [`MIN_GROWTH_INSTRUCTIONS`].
fn grew_beyond_both(first: u64, count: u64, percent: u64) -> bool {
    let ceiling = u128::from(first) * u128::from(100 + percent);
    let above_percent = u128::from(count) * 100 > ceiling;
    above_percent && count.saturating_sub(first) > MIN_GROWTH_INSTRUCTIONS
}

fn series(measured: &[AtSize], phase: &str) -> Result<PhaseSeries, String> {
    let counts = measured.iter().map(|size| {
        size.phases
            .get(phase)
            .copied()
            .ok_or_else(|| format!("scaling: phase {phase} has no count at {} rows", size.rows))
    });
    Ok(PhaseSeries {
        phase: phase.to_owned(),
        counts: counts.collect::<Result<_, _>>()?,
    })
}

fn extra_phase(first: &AtSize, size: &AtSize) -> Option<String> {
    let extra = size
        .phases
        .keys()
        .find(|phase| !first.phases.contains_key(*phase))?;
    Some(format!(
        "scaling: phase {extra} has a count at {} rows and none at {} rows",
        size.rows, first.rows
    ))
}

#[cfg(test)]
#[path = "scaling_tests.rs"]
mod tests;

#[path = "scaling_flat_tests.rs"]
mod flat_tests;
#[cfg(test)]
#[path = "scaling_relaxed_tests.rs"]
mod relaxed_tests;
