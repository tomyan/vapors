//! The guard every output of ux-review passes through.

use super::*;

const KEY: &str = "sk-test-0123456789abcdefghijklmnopqrstuv";

fn secret() -> Secret {
    Secret::new(KEY.to_owned()).expect("a key long enough to guard")
}

#[test]
fn text_without_the_key_passes_unchanged() {
    // Given
    let text = "gpt-5: 23,104 input tokens, $0.04";

    // When
    let passed = secret().guard("the report", text);

    // Then
    assert_eq!(passed, Ok(text));
}

#[test]
fn text_holding_the_whole_key_is_refused_naming_the_output_not_the_key() {
    // Given
    let text = format!("Authorization: Bearer {KEY}");

    // When
    let refused = secret()
        .guard("the PR comment", &text)
        .expect_err("refused");

    // Then
    assert!(refused.contains("the PR comment"), "{refused}");
    assert!(
        !refused.contains(&KEY[..RUN]),
        "the refusal leaks the key: {refused}"
    );
}

#[test]
fn text_holding_a_run_of_the_key_is_refused() {
    // Given an API error that echoes the key's last characters
    let text = format!(
        "Incorrect API key provided: sk-...{}",
        &KEY[KEY.len() - RUN..]
    );

    // When
    let refused = secret().guard("stderr", &text);

    // Then
    assert!(refused.is_err());
}

#[test]
fn a_run_shorter_than_the_guarded_length_passes() {
    // Given
    let text = format!("ends {}", &KEY[KEY.len() - (RUN - 1)..]);

    // When
    let passed = secret().guard("stdout", &text);

    // Then
    assert!(passed.is_ok());
}

#[test]
fn a_key_too_short_to_guard_by_runs_is_refused() {
    // Given
    let short = "sk-short-key".to_owned();

    // When
    let refused = Secret::new(short).expect_err("refused");

    // Then
    assert!(refused.contains("too short"), "{refused}");
    assert!(
        !refused.contains("sk-short"),
        "the refusal leaks the key: {refused}"
    );
}

#[test]
fn a_secret_debug_prints_no_part_of_the_key() {
    // Given
    let secret = secret();

    // When
    let shown = format!("{secret:?}");

    // Then
    assert!(!shown.contains("sk-test"), "{shown}");
}
