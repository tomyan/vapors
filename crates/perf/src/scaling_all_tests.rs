//! The verdict over several changes.

use super::{changes_line, error_block, result_line, verdict};
use crate::rows::RowChange;

#[test]
fn no_failures_pass() {
    assert_eq!(verdict(9, &[]), Ok(()));
}

#[test]
fn one_change_judged_fails_with_its_own_message() {
    // Given
    let failure = "scaling: Insert does work that depends on the page's size".to_owned();

    // When
    let result = verdict(1, std::slice::from_ref(&failure));

    // Then
    assert_eq!(result, Err(failure));
}

#[test]
fn several_changes_judged_fail_naming_the_count_and_every_failure() {
    // Given
    let failures = ["Insert grows".to_owned(), "Resize grows".to_owned()];

    // When
    let error = verdict(9, &failures).expect_err("two failed");

    // Then
    assert_eq!(
        error,
        "scaling: 2 of 9 changes failed:\n\nInsert grows\n\nResize grows"
    );
}

#[test]
fn each_change_ends_with_a_line_naming_it_and_its_result() {
    assert_eq!(
        result_line(RowChange::FlexText, true),
        "scaling-result: FlexText pass"
    );
    assert_eq!(
        result_line(RowChange::Insert, false),
        "scaling-result: Insert fail"
    );
}

/// The log `scripts/nightly-scaling-issues-check.sh` parses: its lines must be
/// the ones this harness prints, so a change of wording fails here.
const FIXTURE: &str = include_str!("../../../scripts/fixtures/nightly-scaling.log");

#[test]
fn the_nightly_fixture_holds_the_lines_the_harness_prints() {
    // Given
    let printed = [
        changes_line(&RowChange::ALL),
        result_line(RowChange::Idle, true),
        result_line(RowChange::Class, false),
        error_block(RowChange::Class, "scaling: Class, add frames does work"),
        error_block(RowChange::Text, "no instruction counter"),
    ];

    // Then
    for line in printed.iter().flat_map(|block| block.lines()) {
        let line = line.split(" does work").next().unwrap_or(line);
        assert!(FIXTURE.contains(line), "fixture lacks `{line}`");
    }
}
