//! The test tiers, split on the test function's name: unit, gpu, host,
//! and pmu, which `ci` never runs.

use super::Step;

// GPU tests are the tests GPU_TESTS selects by nextest id: a function
// named `gpu_`, and in a crate whose integration tests are one binary
// every test in a `tests/gpu_*.rs` file, since the file is the id's
// first segment. The tiers split on the id because nextest refuses a
// `binary()` filter that matches no binary, and a repository may have
// no GPU tests yet. Each tier sets this variable to its name.
// gpu_harness::gpu() refuses "unit", so a GPU test the filter does not
// select fails here on a machine with a GPU, as it would on a runner
// without. A test in a `gpu_*.rs` file that opens no GPU runs in the
// gpu tier only, unchecked by that refusal. vapors-perf's counter opens
// refuse every tier but "pmu" (crates/perf/src/counter_tier.rs), so a test
// that reads a hardware counter without the `pmu_` prefix fails on this
// machine, which has the counters, as it would on a runner without.
pub(super) const TIER_ENV: &str = "VAPORS_TEST_TIER";

/// The GPU tests: a nextest id beginning `gpu_` (a test in a root `gpu_`
/// module, or a whole crate binary's file named so), or one whose
/// function name begins `gpu_` (a crate whose integration tests are one
/// binary, `scripts/one-test-binary.sh`, has ids `<file>::<function>`).
pub(super) const GPU_TESTS: &str = "test(/^gpu_/) | test(/::gpu_[^:]*$/)";

/// Tests that read hardware performance counters (cycles, cache misses),
/// named `pmu_` as the GPU tests are `gpu_`. A machine or VM without
/// those events cannot run them, so they are in no tier `ci` or ci.yml
/// runs: `cargo xtask test --tier pmu` runs them, in a quiet window. A
/// name both filters select (`gpu_module::pmu_test`) is a PMU test: the
/// gpu and host tiers leave it out.
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

/// The private SDK (API builder) harness's tests: xtask's `api_` modules.
/// Their host tests are in no tier `ci`, ci.yml or the gate runs (Tom,
/// 2026-11-25: SDK adoption deprioritised; its harness tests leave routine
/// public CI). Relaxed guard: on GitHub's runner `AppArmor` refuses their
/// bwrap user namespace (run 45482514993), and nothing routine checks
/// them now. `cargo xtask test --tier sdk-host` runs them, on a host that
/// lets bwrap make a user namespace. Retire this when the harness leaves
/// vapors (the lead's runner extraction) or SDK work resumes. Their unit
/// tests stay in the unit tier.
pub(super) const SDK_HARNESS_TESTS: &str = "package(xtask) and test(/^api_/)";

pub(super) fn unit() -> Step {
    tier(
        "unit",
        &format!("not ({GPU_TESTS}) and not ({PMU_TESTS}) and not ({HOST_TESTS})"),
        "run the failing test alone with `cargo nextest run <name>`",
    )
}

pub(super) fn gpu() -> Step {
    tier(
        "gpu",
        &format!("({GPU_TESTS}) and not ({PMU_TESTS})"),
        "needs a Vulkan adapter; the failure names what was found and how to select one",
    )
}

pub(super) fn host() -> Step {
    tier(
        "host",
        &format!("({HOST_TESTS}) and not ({PMU_TESTS}) and not ({SDK_HARNESS_TESTS})"),
        "needs bwrap able to make a user namespace; run the failing test alone with `cargo nextest run <name>`",
    )
}

/// The SDK harness's host tests, which the host tier leaves out.
pub(super) fn sdk_host() -> Step {
    tier(
        "sdk-host",
        &format!("({HOST_TESTS}) and ({SDK_HARNESS_TESTS}) and not ({PMU_TESTS})"),
        "needs bwrap able to make a user namespace; run the failing test alone with `cargo nextest run <name>`",
    )
}

pub(super) fn pmu() -> Step {
    tier(
        "pmu",
        PMU_TESTS,
        "needs Linux perf events for instructions, cycles and cache misses (`sudo sysctl \
         kernel.perf_event_paranoid=1`) and a Vulkan adapter; run in a quiet window",
    )
}

/// The tier `name`: nextest on the tests `filter` selects, with
/// [`TIER_ENV`] set to `name`.
fn tier(name: &str, filter: &str, fix: &str) -> Step {
    let mut step = Step::new(
        name,
        "cargo",
        &["nextest", "run", "--workspace", "-E", filter],
        fix,
    );
    step.env.push((TIER_ENV.to_owned(), name.to_owned()));
    step
}

#[cfg(test)]
#[path = "steps_tiers_tests.rs"]
mod tests;
