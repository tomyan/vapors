//! The guard on the hardware counters' opens ([`InstructionCounter::open`]
//! and [`HardwareCounters::open`]), as `gpu_harness::allowed_in` guards
//! the GPU. xtask's unit, gpu and host tiers set `VAPORS_TEST_TIER` to
//! their name (`xtask/src/steps_tiers.rs`) and the opens refuse there, so
//! a test that reads a counter without the `pmu_` prefix fails on a
//! machine with a PMU too, not only on a hosted runner without one.
//! Unset (a run of `vapors-perf`, or `cargo nextest run` by hand) or
//! [`COUNTER_TIER`], the opens go ahead. Unset is allowed because
//! `vapors-perf` and `cargo xtask scaling` read counters outside any
//! tier; a hosted job that ran nextest without xtask would get no guard
//! (its `pmu_` tests would still fail loudly without a PMU). Setting the
//! variable from `vapors-perf`'s callers would let unset be refused.
//!
//! [`InstructionCounter::open`]: crate::instructions::InstructionCounter::open
//! [`HardwareCounters::open`]: crate::hardware_counters::HardwareCounters::open

use vapors_gpu_harness::TIER_ENV;

/// The value of `VAPORS_TEST_TIER` in which a counter may be opened: the
/// pmu tier's.
pub const COUNTER_TIER: &str = "pmu";

/// Whether this process may open a counter, by `VAPORS_TEST_TIER`.
///
/// # Errors
/// The variable names another tier, or is not Unicode.
pub fn allowed() -> Result<(), String> {
    match std::env::var(TIER_ENV) {
        Ok(tier) => allowed_in(Some(&tier)),
        Err(std::env::VarError::NotPresent) => allowed_in(None),
        Err(std::env::VarError::NotUnicode(raw)) => allowed_in(Some(&raw.to_string_lossy())),
    }
}

/// Whether a counter may be opened in `tier`.
///
/// # Errors
/// `tier` is set and is not [`COUNTER_TIER`].
pub fn allowed_in(tier: Option<&str>) -> Result<(), String> {
    match tier {
        None | Some(COUNTER_TIER) => Ok(()),
        Some(tier) => Err(format!(
            "a test opened a hardware counter in the {tier} tier ({TIER_ENV}={tier}); the pmu \
             tier runs test functions named pmu_*, and a hosted runner may have no PMU, so this \
             test fails there. Fix: rename the test function to pmu_<name>, then run `cargo \
             xtask test --tier pmu`; if it is named pmu_ already, run it that way, not with \
             {TIER_ENV} set to another tier"
        )),
    }
}

#[cfg(test)]
#[path = "counter_tier_tests.rs"]
mod tests;
