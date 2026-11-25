//! The tiers' environment, and its tie to `gpu_harness::gpu()`'s refusal.

use super::*;

#[test]
fn each_tier_tells_its_tests_its_name() {
    for (step, name) in [
        (unit(), "unit"),
        (gpu(), "gpu"),
        (host(), "host"),
        (sdk_host(), "sdk-host"),
        (pmu(), "pmu"),
    ] {
        // Then
        assert_eq!(
            step.env,
            [(TIER_ENV.to_owned(), name.to_owned())],
            "{}",
            step.name
        );
    }
}

// One answer to "which tier refuses the GPU": what unit() sets is the
// variable and value gpu-harness declares (TIER_ENV, REFUSED_TIER).
#[test]
fn the_unit_steps_setting_is_the_one_gpu_harness_refuses() {
    // Given
    let harness = harness_source();
    let (name, value) = unit().env.into_iter().next().expect("unit() sets its tier");

    // Then
    for declared in [
        format!("pub const TIER_ENV: &str = \"{name}\";"),
        format!("pub const REFUSED_TIER: &str = \"{value}\";"),
    ] {
        assert!(
            harness.contains(&declared),
            "gpu-harness lacks `{declared}`"
        );
    }
}

// One answer to "which tier may open a hardware counter": what pmu()
// sets is the value vapors-perf's guard allows (counter_tier.rs).
#[test]
fn the_pmu_steps_setting_is_the_one_the_counter_guard_allows() {
    // Given
    let guard = source("crates/perf/src/counter_tier.rs");
    let (_, value) = pmu().env.into_iter().next().expect("pmu() sets its tier");

    // Then
    let declared = format!("pub const COUNTER_TIER: &str = \"{value}\";");
    assert!(
        guard.contains(&declared),
        "counter_tier.rs lacks `{declared}`"
    );
}

fn harness_source() -> String {
    source("crates/gpu-harness/src/lib.rs")
}

/// A vapors file: in vapors under the root, in touche in the vapors
/// checkout beside it.
fn source(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let places = [root.join(path), root.join("../vapors").join(path)];
    places
        .iter()
        .find_map(|place| std::fs::read_to_string(place).ok())
        .unwrap_or_else(|| panic!("{path} is at none of {places:?}"))
}

#[test]
fn the_unit_tier_leaves_out_the_gpu_pmu_and_host_tests() {
    // When
    let step = unit();

    // Then
    assert_eq!(
        step.args.last().map(String::as_str),
        Some(format!("not ({GPU_TESTS}) and not ({PMU_TESTS}) and not ({HOST_TESTS})").as_str())
    );
}

// A test named like `gpu_module::pmu_test` matches both filters; it reads
// a counter, so the pmu tier runs it and the gpu and host tiers leave it.
#[test]
fn the_gpu_tier_leaves_out_the_pmu_tests() {
    for (step, tests) in [(gpu(), GPU_TESTS)] {
        // Then
        assert_eq!(
            step.args.last().map(String::as_str),
            Some(format!("({tests}) and not ({PMU_TESTS})").as_str())
        );
    }
}

#[test]
fn the_pmu_tier_runs_only_the_pmu_tests_and_is_not_in_ci() {
    // When
    let step = pmu();

    // Then
    assert_eq!(step.args.last().map(String::as_str), Some(PMU_TESTS));
    assert!(!super::super::ci("web").iter().any(|s| s.name == "pmu"));
}

// The SDK harness's host tests fail on GitHub's runner (AppArmor refuses
// bwrap's user namespace); routine CI runs the rest of the host tier.
#[test]
fn the_host_tier_leaves_out_the_pmu_and_sdk_harness_tests() {
    // When
    let step = host();

    // Then
    assert_eq!(
        step.args.last().map(String::as_str),
        Some(
            format!("({HOST_TESTS}) and not ({PMU_TESTS}) and not ({SDK_HARNESS_TESTS})").as_str()
        )
    );
}

#[test]
fn the_sdk_host_tier_runs_only_the_sdk_harness_host_tests_and_is_not_in_ci() {
    // When
    let step = sdk_host();

    // Then
    assert_eq!(
        step.args.last().map(String::as_str),
        Some(format!("({HOST_TESTS}) and ({SDK_HARNESS_TESTS}) and not ({PMU_TESTS})").as_str())
    );
    assert!(!super::super::ci("web").iter().any(|s| s.name == "sdk-host"));
}
