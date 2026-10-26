//! The money side of a live review against the local stand-in: what a
//! request leaves in the day's spend when it settles, and when it never
//! does (review-v69 H1).

use std::path::Path;
use std::time::SystemTime;

use super::run;
use super::tests::{SHA, answered, counted, prepared, report_dir, secret, ux};
use crate::ux_stub_tests::stand_in_many;

fn spent_today(dir: &Path) -> f64 {
    let today = &crate::ux_ledger::utc(SystemTime::now())[..10];
    crate::ux_ledger::spent_on(&dir.join(crate::ux_ledger::FILE), today).expect("read")
}

/// The `ux()` prices: 15,000 counted tokens at $2 per million.
const COUNT_COST: f64 = 0.03;
/// The worst case booked: 15,000 at $2 and 12,000 output at $12.
const WORST: f64 = 0.174;

#[test]
fn a_settled_review_counts_its_count_and_its_usage() {
    // Given
    let dir = report_dir("settled");
    let (url, _seen) = stand_in_many(vec![counted(15_000), answered("first-screen", 15_000)]);

    // When
    run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect("reviewed");

    // Then: the count, and 15,000 at $2 plus 500 at $12 in place of the worst case
    assert!(
        (spent_today(&dir) - (COUNT_COST + 0.036)).abs() < 1e-9,
        "{}",
        spent_today(&dir)
    );
}

#[test]
fn a_connection_closed_after_the_request_counts_at_its_worst_case() {
    // Given: the stand-in reads the billed request, then closes the socket
    let dir = report_dir("closed");
    let (url, _seen) = stand_in_many(vec![counted(15_000), ("close", String::new())]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(refused.starts_with("curl failed"), "{refused}");
    assert!(
        (spent_today(&dir) - (COUNT_COST + WORST)).abs() < 1e-9,
        "{}",
        spent_today(&dir)
    );
}

#[test]
fn a_response_with_no_usage_counts_at_its_worst_case() {
    // Given
    let dir = report_dir("no-usage");
    let (url, _seen) = stand_in_many(vec![
        counted(15_000),
        (
            "200 OK",
            r#"{"status": "completed", "output": []}"#.to_owned(),
        ),
    ]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(refused.contains("no `input_tokens`"), "{refused}");
    assert!(
        (spent_today(&dir) - (COUNT_COST + WORST)).abs() < 1e-9,
        "{}",
        spent_today(&dir)
    );
}
