//! A watched part that overruns fails naming itself; one that ends in
//! time does not.

use std::sync::mpsc::channel;
use std::time::Duration;

use super::*;

#[test]
fn a_part_still_running_at_its_deadline_fails_naming_the_part() {
    // Given
    let (said, line) = channel();

    // When: the part is still running at its 100 ms deadline
    let watched = watch_with("the settings", Duration::from_millis(100), move |line| {
        said.send(line).expect("said");
    });
    let failure = line.recv_timeout(Duration::from_secs(5));
    drop(watched);

    // Then
    let failure = failure.expect("the watchdog said nothing");
    assert!(
        failure.starts_with("FAIL gate: the settings did not finish within 100ms")
            && failure.contains("cargo xtask gate"),
        "{failure}"
    );
}

#[test]
fn a_part_that_ends_in_time_is_not_failed() {
    // Given
    let (said, line) = channel();

    // When: the part ends at once, well inside its 1 s deadline
    drop(watch_with(
        "the settings",
        Duration::from_secs(1),
        move |line| {
            said.send(line).expect("said");
        },
    ));

    // Then: the watch thread ends without a word (its sender dropped)
    assert!(line.recv_timeout(Duration::from_secs(5)).is_err());
}
