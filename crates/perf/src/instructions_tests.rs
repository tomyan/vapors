//! The refusal when perf events cannot be opened.

use super::refusal;

#[test]
fn the_refusal_names_the_sysctl_and_the_value_it_needs() {
    // Given
    let why = "perf_event_open failed: denied";

    // When
    let message = refusal(why);

    // Then
    assert!(message.contains("perf_event_paranoid"), "{message}");
    assert!(message.contains("2 or lower"), "{message}");
    assert!(
        message.contains("sudo sysctl kernel.perf_event_paranoid=1"),
        "{message}"
    );
}
