use super::*;

const LABEL: &str = "gpu / install pinned tools";

#[test]
fn a_literal_value_is_used_as_written() {
    // When
    let value = env_text("llvmpipe", "VAPORS_GPU_ADAPTER", LABEL, None);

    // Then
    assert_eq!(value, Ok(Some("llvmpipe".to_owned())));
}

#[test]
fn the_github_token_is_the_local_one_when_set() {
    // When
    let value = env_text(
        "${{ github.token }}",
        "GITHUB_TOKEN",
        LABEL,
        Some("ghp_local"),
    );

    // Then
    assert_eq!(value, Ok(Some("ghp_local".to_owned())));
}

#[test]
fn the_github_token_is_left_unset_when_there_is_no_local_one() {
    // When
    let value = env_text("${{ github.token }}", "GITHUB_TOKEN", LABEL, None);

    // Then
    assert_eq!(value, Ok(None));
}

#[test]
fn the_note_for_an_unset_token_says_the_step_runs_unauthenticated() {
    // When
    let note = unauthenticated_note("GITHUB_TOKEN", LABEL);

    // Then
    assert!(note.contains(LABEL), "{note}");
    assert!(note.contains("GITHUB_TOKEN is not set"), "{note}");
    assert!(note.contains("runs unauthenticated"), "{note}");
}

#[test]
fn every_other_expression_is_refused_naming_the_step_and_variable() {
    for text in [
        "${{ secrets.GITHUB_TOKEN }}",
        "${{github.token}}",
        "Bearer ${{ github.token }}",
        "${{ github.sha }}",
    ] {
        // When
        let refusal = env_text(text, "TOKEN", LABEL, Some("ghp_local")).expect_err(text);

        // Then
        assert!(refusal.starts_with(LABEL), "{refusal}");
        assert!(refusal.contains("env TOKEN is an expression"), "{refusal}");
        assert!(refusal.contains("${{ github.token }}"), "{refusal}");
    }
}
