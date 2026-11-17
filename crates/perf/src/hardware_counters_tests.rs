//! The refusal when the counter group cannot be opened, and the reading
//! checks: a multiplexed reading and one without its times are refused.

use super::{checked, refusal};

#[test]
fn the_refusal_names_the_events_and_the_exact_sysctl_command() {
    // Given
    let why = "perf_event_open of LLC demand read misses failed: Permission denied";

    // When
    let message = refusal(why);

    // Then
    assert!(message.starts_with("--counters needs Linux perf events for cycles, instructions, branch misses, L1d misses, LLC demand read misses: "), "{message}");
    assert!(message.contains(why), "{message}");
    assert!(
        message.contains("`sudo sysctl kernel.perf_event_paranoid=1`"),
        "{message}"
    );
}

#[test]
fn a_reading_running_for_the_whole_time_enabled_is_its_counts() {
    // When
    let reading = checked(([1, 2, 3, 4, 5], Some(900), Some(900)));

    // Then
    assert_eq!(reading, Ok([1, 2, 3, 4, 5]));
}

#[test]
fn a_multiplexed_reading_is_refused() {
    // When: on the PMU for less time than it was enabled
    let refused = checked(([1, 2, 3, 4, 5], Some(900), Some(600))).expect_err("multiplexed");

    // Then
    assert!(
        refused.contains("not running for the whole time"),
        "{refused}"
    );
}

#[test]
fn a_reading_without_its_times_is_refused_naming_the_missing_one() {
    for (enabled, running, missing) in [
        (None, Some(900), "time enabled"),
        (Some(900), None, "time running"),
        (None, None, "time enabled"),
    ] {
        // When
        let refused = checked(([0; 5], enabled, running)).expect_err("no time");

        // Then: a missing time is not a zero that passes the check
        assert!(refused.contains(missing), "{refused}");
        assert!(refused.contains("TOTAL_TIME_ENABLED"), "{refused}");
    }
}
