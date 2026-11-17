//! An event that does not rise over work that must raise it is refused,
//! naming it; the work itself is checked on a real PMU by the pmu tier
//! (`tests/pmu_counters_frames.rs` opens the group).

use super::rose;
use crate::hardware_counters::{L1D_MISSES, LLC_MISSES};

#[test]
fn an_event_that_rose_enough_passes() {
    // Then
    assert_eq!(rose(L1D_MISSES, [0; 5], [0, 0, 0, 600, 0], 500), Ok(()));
}

#[test]
fn an_event_that_stayed_at_zero_is_refused_naming_it() {
    // When
    let refused = rose(LLC_MISSES, [0; 5], [9, 9, 9, 9, 0], 500).expect_err("flat");

    // Then
    assert!(
        refused.starts_with("--counters: LLC demand read misses rose by 0 over work that must raise it by at least 500"),
        "{refused}"
    );
}
