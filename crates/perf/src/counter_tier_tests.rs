//! Which tiers may open a hardware counter.

use super::{COUNTER_TIER, allowed_in};

#[test]
fn a_counter_opens_outside_the_tiers_and_in_the_pmu_tier() {
    // Then
    assert_eq!(allowed_in(None), Ok(()));
    assert_eq!(allowed_in(Some(COUNTER_TIER)), Ok(()));
}

#[test]
fn a_counter_in_another_tier_is_refused_naming_the_tier_and_the_rename() {
    for tier in ["unit", "gpu", "host", "somewhere"] {
        // When
        let refusal = allowed_in(Some(tier)).expect_err(tier);

        // Then
        assert!(
            refusal.contains(&format!("VAPORS_TEST_TIER={tier}")),
            "{refusal}"
        );
        assert!(
            refusal.contains("rename the test function to pmu_<name>"),
            "{refusal}"
        );
        assert!(refusal.contains("cargo xtask test --tier pmu"), "{refusal}");
    }
}
