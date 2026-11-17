//! The test tiers, split on the test function's name: unit, gpu, host,
//! and pmu, which `ci` never runs.

use super::Step;

// GPU tests are the tests GPU_TESTS selects by nextest id: a function
// named `gpu_`, and in a crate whose integration tests are one binary
// every test in a `tests/gpu_*.rs` file, since the file is the id's
// first segment. The tiers split on the id because nextest refuses a
// `binary()` filter that matches no binary, and a repository may have
// no GPU tests yet. The unit tier sets this variable and
// gpu_harness::gpu() refuses it, so a GPU test the filter does not
// select fails here on a machine with a GPU, as it would on a runner
// without. A test in a `gpu_*.rs` file that opens no GPU runs in the
// gpu tier only, unchecked by that refusal.
pub(super) const UNIT_TIER: (&str, &str) = ("VAPORS_TEST_TIER", "unit");

/// The GPU tests: a nextest id beginning `gpu_` (a test in a root `gpu_`
/// module, or a whole crate binary's file named so), or one whose
/// function name begins `gpu_` (a crate whose integration tests are one
/// binary, `scripts/one-test-binary.sh`, has ids `<file>::<function>`).
pub(super) const GPU_TESTS: &str = "test(/^gpu_/) | test(/::gpu_[^:]*$/)";

/// Tests that read hardware performance counters (cycles, cache misses),
/// named `pmu_` as the GPU tests are `gpu_`. A machine or VM without
/// those events cannot run them, so they are in no tier `ci` or ci.yml
/// runs: `cargo xtask test --tier pmu` runs them, in a quiet window.
pub(super) const PMU_TESTS: &str = "test(/^pmu_/) | test(/::pmu_[^:]*$/)";

/// Tests that use what only the host has, named `host_`: bwrap making a
/// user namespace, which an API builder's sandbox forbids
/// (`--disable-userns`, `docs/plans/api-builder.md`), and the user's
/// systemd. `ci` and ci.yml run them in their own tier, so the gate does
/// (each gate step runs inside the gate's bwrap, the host tier
/// included). An API builder's sandboxed ci will run every tier but this
/// one, once `request_push` lands (the plan's slice 4); until then a
/// sandboxed ci fails at this step, loudly. Each has a unit-tier twin
/// that gives the host's answer instead of asking for it.
pub(super) const HOST_TESTS: &str = "test(/^host_/) | test(/::host_[^:]*$/)";

pub(super) fn unit() -> Step {
    let mut step = Step::new(
        "unit",
        "cargo",
        &[
            "nextest",
            "run",
            "--workspace",
            "-E",
            &format!("not ({GPU_TESTS}) and not ({PMU_TESTS}) and not ({HOST_TESTS})"),
        ],
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
        &["nextest", "run", "--workspace", "-E", GPU_TESTS],
        "needs a Vulkan adapter; the failure names what was found and how to select one",
    )
}

pub(super) fn host() -> Step {
    Step::new(
        "host",
        "cargo",
        &["nextest", "run", "--workspace", "-E", HOST_TESTS],
        "needs bwrap able to make a user namespace; run the failing test alone with `cargo nextest run <name>`",
    )
}

pub(super) fn pmu() -> Step {
    Step::new(
        "pmu",
        "cargo",
        &["nextest", "run", "--workspace", "-E", PMU_TESTS],
        "needs Linux perf events for cycles and cache misses (`sudo sysctl \
         kernel.perf_event_paranoid=1`) and a Vulkan adapter; run in a quiet window",
    )
}

#[cfg(test)]
#[path = "steps_tiers_tests.rs"]
mod tests;
