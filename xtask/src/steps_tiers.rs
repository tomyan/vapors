//! The test tiers, split on the test function's name: unit, gpu, and
//! pmu, which `ci` never runs.

use super::Step;

// GPU tests are test functions whose names start with `gpu_` (kept in
// `tests/gpu_*.rs` files); the tiers split on the function name because
// nextest refuses a `binary()` filter that matches no binary, and a
// repository may have no GPU tests yet. The unit tier sets this variable
// and gpu_harness::gpu() refuses it, so a GPU test without the prefix
// fails here on a machine with a GPU, as it would on a runner without.
pub(super) const UNIT_TIER: (&str, &str) = ("VAPORS_TEST_TIER", "unit");

// Tests that read hardware performance counters (cycles, cache misses)
// start with `pmu_`. A machine or VM without those events cannot run
// them, so they are in no tier `ci` or ci.yml runs: `cargo xtask test
// --tier pmu` runs them, in a quiet window.
const UNIT_FILTER: &str = "not test(/^gpu_/) and not test(/^pmu_/)";

pub(super) fn unit() -> Step {
    let mut step = Step::new(
        "unit",
        "cargo",
        &["nextest", "run", "--workspace", "-E", UNIT_FILTER],
        "run the failing test alone with `cargo nextest run <name>`",
    );
    step.env
        .push((UNIT_TIER.0.to_owned(), UNIT_TIER.1.to_owned()));
    step
}

pub(super) fn gpu() -> Step {
    Step::new(
        "gpu",
        "cargo",
        &["nextest", "run", "--workspace", "-E", "test(/^gpu_/)"],
        "needs a Vulkan adapter; the failure names what was found and how to select one",
    )
}

pub(super) fn pmu() -> Step {
    Step::new(
        "pmu",
        "cargo",
        &["nextest", "run", "--workspace", "-E", "test(/^pmu_/)"],
        "needs Linux perf events for cycles and cache misses (`sudo sysctl \
         kernel.perf_event_paranoid=1`) and a Vulkan adapter; run in a quiet window",
    )
}

#[cfg(test)]
#[path = "steps_tiers_tests.rs"]
mod tests;
