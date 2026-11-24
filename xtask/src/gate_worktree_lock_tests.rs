//! One gate at a time in a worktree.

use std::time::{Duration, Instant};

use super::*;

const BRIEF: Duration = Duration::from_millis(300);

#[test]
fn a_second_gate_in_the_worktree_waits_then_refuses_naming_the_first() {
    // Given
    let dir = crate::scratch::dir("worktree-lock-second");
    let _first = take(&dir, "/w/here", BRIEF).expect("first");

    // When
    let started = Instant::now();
    let refusal = take(&dir, "/w/here", BRIEF).expect_err("held");

    // Then
    assert!(started.elapsed() >= BRIEF, "it did not wait");
    let first = format!("pid {} in /w/here for ", std::process::id());
    assert!(
        refusal.contains(&first) && refusal.contains("rerun `cargo xtask gate`"),
        "{refusal}"
    );
}
