//! The tiers' environment, and its tie to `gpu_harness::gpu()`'s refusal.

use super::*;

#[test]
fn the_unit_tier_tells_its_tests_it_is_the_unit_tier() {
    // When
    let step = unit();

    // Then
    let expected = (UNIT_TIER.0.to_owned(), UNIT_TIER.1.to_owned());
    assert_eq!(step.env, [expected]);
}

#[test]
fn the_gpu_tier_sets_no_tier_so_gpu_opens_the_gpu() {
    // When
    let step = gpu();

    // Then
    assert!(
        !step.env.iter().any(|(name, _)| name == UNIT_TIER.0),
        "{:?}",
        step.env
    );
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

/// gpu-harness's lib.rs: in vapors under crates/, in touche in the
/// vapors checkout beside it.
fn harness_source() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let places = [
        root.join("crates/gpu-harness/src/lib.rs"),
        root.join("../vapors/crates/gpu-harness/src/lib.rs"),
    ];
    places
        .iter()
        .find_map(|place| std::fs::read_to_string(place).ok())
        .unwrap_or_else(|| panic!("gpu-harness's lib.rs is at none of {places:?}"))
}

#[test]
fn the_unit_tier_leaves_out_the_gpu_and_pmu_tests() {
    let step = unit();
    assert_eq!(
        step.args.last().map(String::as_str),
        Some("not test(/^gpu_/) and not test(/^pmu_/)")
    );
}

#[test]
fn the_pmu_tier_runs_only_the_pmu_tests_and_is_not_in_ci() {
    // When
    let step = pmu();

    // Then
    assert_eq!(step.args.last().map(String::as_str), Some("test(/^pmu_/)"));
    assert!(!super::super::ci("web").iter().any(|s| s.name == "pmu"));
}
