use super::*;

#[test]
fn the_unit_tier_is_refused_naming_the_rename() {
    // When
    let refusal = allowed_in(Some("unit")).unwrap_err();

    // Then
    assert!(refusal.contains("VAPORS_TEST_TIER=unit"), "{refusal}");
    assert!(
        refusal.contains("rename the test function to gpu_<name>"),
        "{refusal}"
    );
}

#[test]
fn the_gpu_tier_and_a_direct_run_may_open_a_gpu() {
    for tier in [Some("gpu"), None] {
        // When
        let allowed = allowed_in(tier);

        // Then
        assert_eq!(allowed, Ok(()), "{tier:?}");
    }
}

// One answer to "which tier is this": xtask's unit step sets the
// variable gpu() reads, to the value it refuses.
#[test]
fn xtasks_unit_step_sets_the_tier_gpu_refuses() {
    // Given
    let steps = concat!(env!("CARGO_MANIFEST_DIR"), "/../../xtask/src/steps.rs");

    // When
    let source = std::fs::read_to_string(steps).expect(steps);

    // Then
    let setting = format!("(\"{TIER_ENV}\", \"unit\")");
    assert!(source.contains(&setting), "{steps} sets no {setting}");
}
