//! Lines written whole, and the ledger held by one review at a time.

use std::time::Duration;

use super::*;

fn dir(test: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "xtask-ux-ledger-write-{test}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn each_line_is_written_whole_and_ends_the_file() {
    // Given
    let dir = dir("whole");
    let entry = Entry {
        time: "2026-10-26T01:00:00Z",
        sha: "abc",
        model: "gpt-6.1-sol",
        id: "x",
    };

    // When
    sent(&dir.join("spend.jsonl"), &entry, 0.18).expect("written");
    count(&dir.join("spend.jsonl"), &entry, 10, 0.00002).expect("written");

    // Then
    let text = std::fs::read_to_string(dir.join("spend.jsonl")).expect("read");
    assert_eq!(text.lines().count(), 2);
    assert!(text.ends_with('\n'));
}

#[test]
fn a_second_hold_waits_and_then_is_refused_naming_the_lock() {
    // Given
    let dir = dir("held");
    let first = hold(&dir, Duration::from_secs(1)).expect("held");

    // When
    let refused = hold(&dir, Duration::from_millis(300))
        .err()
        .expect("refused");

    // Then
    assert!(refused.contains("spend.lock"), "{refused}");
    drop(first);
    assert!(
        hold(&dir, Duration::from_millis(300)).is_ok(),
        "released on drop"
    );
}
