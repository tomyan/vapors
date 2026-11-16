use std::process::Command;
use std::time::Duration;

use super::output_within;

#[test]
fn a_child_within_its_deadline_gives_its_output() {
    // When
    let output =
        output_within(Command::new("echo").arg("hello"), Duration::from_secs(10)).expect("ran");

    // Then
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn a_child_past_its_deadline_is_killed_and_refused_naming_the_deadline() {
    // When
    let refused = output_within(Command::new("sleep").arg("30"), Duration::from_millis(200))
        .expect_err("refused");

    // Then
    assert!(refused.contains("\"sleep\" \"30\""), "{refused}");
    assert!(refused.contains("deadline of 200ms"), "{refused}");
}
