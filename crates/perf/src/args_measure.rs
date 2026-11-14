//! What a full run measures per phase (`args.rs`): times, instructions
//! (`--instructions`) or hardware counts (`--counters`), one of them.

use crate::args::USAGE;

/// What the full run reports per phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Measure {
    /// Wall times, with the GPU's.
    #[default]
    Times,
    /// Instructions retired: `--instructions`.
    Instructions,
    /// Cycles, instructions per cycle, branch and cache misses:
    /// `--counters`, diagnostics for a quiet window.
    Counters,
}

impl Measure {
    /// The flag that asked for it; none for times.
    #[must_use]
    pub fn flag(self) -> Option<&'static str> {
        match self {
            Self::Times => None,
            Self::Instructions => Some("--instructions"),
            Self::Counters => Some("--counters"),
        }
    }

    /// `self` with `asked` given as well.
    ///
    /// # Errors
    /// `--instructions` and `--counters` both given, which `--counters`
    /// makes needless: it counts instructions too.
    pub(crate) fn and(self, asked: Self) -> Result<Self, String> {
        if self == Self::Times || self == asked {
            return Ok(asked);
        }
        Err(format!(
            "--instructions and --counters cannot be combined (--counters counts instructions \
             too): {USAGE}"
        ))
    }
}

/// `--scaling` always counts instructions, over a fixed set of sizes and
/// workloads, so `--only` and `--instructions` or `--counters` have nothing
/// to select or add.
pub(crate) fn refuse_scaling_with_others(
    scaling: bool,
    only: bool,
    measure: Measure,
) -> Result<(), String> {
    if scaling && only {
        return Err(format!("--scaling and --only cannot be combined: {USAGE}"));
    }
    if let (true, Some(flag)) = (scaling, measure.flag()) {
        return Err(format!(
            "--scaling and {flag} cannot be combined (--scaling always counts instructions): {USAGE}"
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "args_measure_tests.rs"]
mod tests;
