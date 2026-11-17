//! The refusal when the counter group cannot be opened.

use super::refusal;

#[test]
fn the_refusal_names_the_events_and_the_exact_sysctl_command() {
    // Given
    let why = "perf_event_open of LLC read misses failed: Permission denied";

    // When
    let message = refusal(why);

    // Then
    assert!(message.starts_with("--counters needs Linux perf events for cycles, instructions, branch misses, L1d read misses, LLC read misses: "), "{message}");
    assert!(message.contains(why), "{message}");
    assert!(
        message.contains("`sudo sysctl kernel.perf_event_paranoid=1`"),
        "{message}"
    );
}
