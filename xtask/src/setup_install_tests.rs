use super::*;

fn sccache() -> Tool {
    Tool {
        crate_name: "sccache".into(),
        binary: "sccache".into(),
        version: "0.18.0".into(),
    }
}

#[test]
fn an_absent_tool_is_reported_as_missing_not_as_a_failure() {
    // When
    let line = before_install(&sccache(), None);

    // Then
    assert_eq!(
        line,
        "missing sccache            0.18.0 not installed; installing"
    );
}

#[test]
fn a_tool_at_another_version_is_reported_with_both_versions() {
    // When
    let line = before_install(&sccache(), Some("0.17.0"));

    // Then
    assert_eq!(
        line,
        "wrong   sccache            found 0.17.0, pinned 0.18.0; installing"
    );
}

#[test]
fn binstall_may_only_download_never_compile() {
    // When
    let args = binstall_args("sccache@0.18.0");

    // Then
    assert_eq!(
        args,
        [
            "binstall",
            "-y",
            "--disable-strategies",
            "compile",
            "sccache@0.18.0"
        ]
    );
}

#[test]
fn a_failed_download_names_the_tool_the_error_and_the_reproducing_command() {
    // Given
    let output = "INFO resolving\nWARN 403 Forbidden from api.github.com\nERROR no binary found\n";

    // When
    let message = failure("sccache@0.18.0", &Ended::Failed(output.into()));

    // Then
    assert!(
        message.starts_with("could not download sccache@0.18.0"),
        "{message}"
    );
    assert!(
        message.contains("WARN 403 Forbidden from api.github.com"),
        "{message}"
    );
    assert!(message.contains("ERROR no binary found"), "{message}");
    assert!(
        message
            .contains("reproduce: cargo binstall -y --disable-strategies compile sccache@0.18.0"),
        "{message}"
    );
}

#[test]
fn a_download_past_the_deadline_says_so_with_the_output_so_far() {
    // Given
    let output = "WARN 403 Forbidden; will wait for 120s and retry\n";

    // When
    let message = failure("sccache@0.18.0", &Ended::TimedOut(output.into()));

    // Then
    assert!(message.contains("did not finish within 60 s"), "{message}");
    assert!(
        message.contains("will wait for 120s and retry"),
        "{message}"
    );
}

#[test]
fn only_the_last_lines_of_a_long_output_are_quoted() {
    // Given
    let output = (1..=40)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n");

    // When
    let message = failure("sccache@0.18.0", &Ended::Failed(output));

    // Then
    assert!(message.contains("line 40"), "{message}");
    assert!(message.contains("line 31"), "{message}");
    assert!(!message.contains("line 30\n"), "{message}");
}
