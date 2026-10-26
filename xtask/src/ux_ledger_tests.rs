//! The spend ledger and the daily ceiling.

use std::path::PathBuf;
use std::time::Duration;

use super::*;

fn ledger(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-ux-ledger-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join(FILE)
}

fn run(time: &str, cost: f64) -> Run<'_> {
    Run {
        time,
        sha: "abc",
        model: "gpt-5.6-terra",
        usage: Usage {
            input: 1,
            cached: 0,
            output: 1,
        },
        cost,
    }
}

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
fn the_day_s_spend_sums_only_that_day_s_runs() {
    // Given
    let path = ledger("sum");
    record(&path, &run("2026-10-25T23:59:59Z", 0.50)).expect("recorded");
    record(&path, &run("2026-10-26T01:00:00Z", 0.25)).expect("recorded");
    record(&path, &run("2026-10-26T02:00:00Z", 0.125)).expect("recorded");

    // When
    let spent = spent_on(&path, "2026-10-26").expect("read");

    // Then
    assert!((spent - 0.375).abs() < 1e-9, "{spent}");
}

#[test]
fn no_ledger_yet_means_nothing_spent() {
    assert_eq!(spent_on(&ledger("none"), "2026-10-26"), Ok(0.0));
}

#[test]
fn a_ledger_line_that_is_not_a_run_is_refused_naming_it() {
    // Given
    let path = ledger("bad");
    record(&path, &run("2026-10-26T01:00:00Z", 0.25)).expect("recorded");
    std::fs::write(
        &path,
        format!(
            "{}{{\"time\": \"2026-10-26\"}}\n",
            std::fs::read_to_string(&path).expect("read")
        ),
    )
    .expect("written");

    // When
    let refused = spent_on(&path, "2026-10-26").expect_err("refused");

    // Then
    assert!(
        refused.ends_with(":2: a run with no `time` or `cost`"),
        "{refused}"
    );
}

#[test]
fn a_run_that_would_pass_the_daily_ceiling_is_refused_with_the_day_s_total() {
    // When
    let refused = within_day(2.80, 0.29, 3.00).expect_err("refused");

    // Then
    assert!(refused.contains("today's spend $2.8000 plus this run's worst case $0.2900 is over ux.daily_cost_usd $3.00"), "{refused}");
    assert_eq!(within_day(2.70, 0.29, 3.00), Ok(()));
}

#[test]
fn a_count_line_adds_nothing_to_the_day_s_spend() {
    // Given a run and a count on the same day
    let path = ledger("count");
    record(&path, &run("2026-10-26T01:00:00Z", 0.25)).expect("recorded");
    record_count(&path, "2026-10-26T01:00:01Z", "abc", "gpt-6.1-sol", 26_924).expect("recorded");

    // When
    let spent = spent_on(&path, "2026-10-26").expect("read");

    // Then
    assert!((spent - 0.25).abs() < 1e-9, "{spent}");
}
