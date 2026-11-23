//! The checkout's state is read within a deadline.

use std::time::Duration;

use super::*;

#[test]
fn a_state_read_past_its_deadline_fails_naming_the_part_and_the_call() {
    // Given: a call that does not end
    let mut command = Command::new("sleep");
    command.arg("30");

    // When
    let refusal = output_of(
        command,
        "git -C . status --porcelain",
        Duration::from_millis(100),
    )
    .expect_err("past its deadline");

    // Then
    assert!(
        refusal.contains("the gate's checkout state, `git -C . status --porcelain`")
            && refusal.contains("deadline")
            && refusal.contains("cargo xtask gate"),
        "{refusal}"
    );
}
