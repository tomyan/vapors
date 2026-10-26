//! The spend ledger and the daily ceiling.

use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::ux_answer::Usage;
use crate::ux_ledger_write::{Entry, count, sent, settled};

fn ledger(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-ux-ledger-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join(FILE)
}

fn entry<'a>(time: &'a str, id: &'a str) -> Entry<'a> {
    Entry {
        time,
        sha: "abc",
        model: "gpt-6.1-sol",
        id,
    }
}

const USAGE: Usage = Usage {
    input: 1,
    cached: 0,
    output: 1,
};

#[test]
fn times_are_written_in_utc() {
    // Given 2026-10-26 07:05:09 UTC
    let at = UNIX_EPOCH + Duration::from_secs(1_792_998_309);

    // When
    let time = utc(at);

    // Then
    assert_eq!(time, "2026-10-26T07:05:09Z");
}

#[test]
#[allow(
    clippy::duration_suboptimal_units,
    reason = "a Unix timestamp reads as seconds"
)]
fn a_leap_day_and_the_epoch_are_dated_right() {
    assert_eq!(utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
    assert_eq!(
        &utc(UNIX_EPOCH + Duration::from_secs(1_709_164_800))[..10],
        "2024-02-29"
    );
}

#[test]
fn the_day_s_spend_sums_counts_and_settled_requests_of_that_day_only() {
    // Given
    let path = ledger("sum");
    settled(&path, &entry("2026-10-25T23:59:59Z", "a"), USAGE, 0.50).expect("recorded");
    count(&path, &entry("2026-10-26T01:00:00Z", ""), 26_924, 0.054).expect("recorded");
    sent(&path, &entry("2026-10-26T01:00:01Z", "b"), 0.18).expect("recorded");
    settled(&path, &entry("2026-10-26T01:00:15Z", "b"), USAGE, 0.06).expect("recorded");

    // When
    let spent = spent_on(&path, "2026-10-26").expect("read");

    // Then: the count, and the settled cost in place of its worst case
    assert!((spent - 0.114).abs() < 1e-9, "{spent}");
}

#[test]
fn a_sent_request_never_settled_counts_at_its_worst_case() {
    // Given a request cut off before its usage was read
    let path = ledger("unsettled");
    sent(&path, &entry("2026-10-26T01:00:01Z", "b"), 0.18).expect("recorded");

    // When
    let spent = spent_on(&path, "2026-10-26").expect("read");

    // Then
    assert!((spent - 0.18).abs() < 1e-9, "{spent}");
}

#[test]
fn no_ledger_yet_means_nothing_spent() {
    assert_eq!(spent_on(&ledger("none"), "2026-10-26"), Ok(0.0));
}

#[test]
fn a_line_of_no_known_kind_is_refused_naming_it() {
    // Given
    let path = ledger("bad");
    sent(&path, &entry("2026-10-26T01:00:01Z", "b"), 0.18).expect("recorded");
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(
        &path,
        format!("{{\"time\": \"2026-10-26\", \"cost\": 1}}\n{text}"),
    )
    .expect("written");

    // When
    let refused = spent_on(&path, "2026-10-26").expect_err("refused");

    // Then
    assert!(
        refused.contains("spend.jsonl:1: a line with no"),
        "{refused}"
    );
}

#[test]
fn a_torn_last_line_is_named_with_its_fix() {
    // Given a write cut short
    let path = ledger("torn");
    sent(&path, &entry("2026-10-26T01:00:01Z", "b"), 0.18).expect("recorded");
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(&path, format!("{text}{{\"time\": \"2026-10")).expect("written");

    // When
    let refused = spent_on(&path, "2026-10-26").expect_err("refused");

    // Then
    assert!(refused.contains(":2: the last line is torn"), "{refused}");
    assert!(refused.contains("fix: delete that line"), "{refused}");
}

#[test]
fn a_run_that_would_pass_the_daily_ceiling_is_refused_with_the_day_s_total() {
    // When
    let refused = within_day(2.80, 0.29, 3.00).expect_err("refused");

    // Then
    assert!(
        refused.contains("today's spend $2.8000 plus this run's worst case $0.2900 is over ux.daily_cost_usd $3.00"),
        "{refused}"
    );
    assert_eq!(within_day(2.70, 0.29, 3.00), Ok(()));
}
