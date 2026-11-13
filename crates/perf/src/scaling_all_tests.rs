//! The verdict over several changes.

use super::verdict;

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
