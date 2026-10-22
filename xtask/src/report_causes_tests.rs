//! Failures whose cause is printed below the classified line: a build
//! script's output under cargo's `Caused by:`, and nextest's tests that
//! died by signal or deadline. The fixtures are captured from real runs
//! (`report_fixtures/`, scratch paths shortened to `/scratch`).

use super::*;

const BUILD_SCRIPT_FAILED: &str = include_str!("report_fixtures/build_script_failed.txt");
const CAUSED_BY_CHAIN: &str = include_str!("report_fixtures/caused_by_chain.txt");
const NEXTEST_SIGNAL_AND_TIMEOUT: &str =
    include_str!("report_fixtures/nextest_signal_and_timeout.txt");

#[test]
fn a_failed_build_script_keeps_its_stdout_and_stderr_blocks() {
    // When
    let lines = lines_to_show(BUILD_SCRIPT_FAILED);

    // Then
    assert_eq!(
        lines[2..],
        [
            "Caused by:",
            "process didn't exit successfully: `/scratch/report-scratch-target/debug/build/\
             scratch-build-2c16db07e015c966/build-script-build` (exit status: 1)",
            "--- stdout",
            "cargo:warning=checking for libfoo",
            "looked in /usr/lib/foo",
            "--- stderr",
            "libfoo not found: install libfoo-dev or set FOO_DIR",
        ]
    );
}

#[test]
fn every_link_of_a_caused_by_chain_is_kept() {
    // When
    let lines = lines_to_show(CAUSED_BY_CHAIN);

    // Then: the second `Caused by:` is a repeat and shown once
    assert_eq!(
        lines[lines.len() - 3..],
        [
            "Caused by:",
            "failed to load source for dependency `missing`",
            "unable to update /scratch/missing-crate",
        ]
    );
}

#[test]
fn a_line_after_the_cause_block_ends_it() {
    // Given: the cause block, then cargo's next unindented line
    let output =
        format!("{BUILD_SCRIPT_FAILED}warning: build failed, waiting for other jobs\nchatter\n");

    // When
    let lines = lines_to_show(&output);

    // Then
    assert_eq!(
        lines.last(),
        Some(&"warning: build failed, waiting for other jobs")
    );
}

#[test]
fn nextest_signal_and_timeout_lines_name_each_test() {
    // When
    let lines = lines_to_show(NEXTEST_SIGNAL_AND_TIMEOUT);

    // Then
    let named: Vec<&str> = lines
        .iter()
        .filter_map(|line| line.split_once(' ').map(|(word, _)| word))
        .filter(|word| ["SIGABRT", "SIGSEGV", "TIMEOUT"].contains(word))
        .collect();
    assert_eq!(named, ["SIGABRT", "SIGSEGV", "TIMEOUT"]);
    assert!(
        lines
            .iter()
            .any(|line| line.ends_with("scratch-build::crash segfaults")),
        "{lines:?}"
    );
}

#[test]
fn nextest_progress_lines_are_not_failures() {
    // Given
    let output = "        SLOW [>  1.000s] (───) a::b\n TERMINATING [>  2.000s] (───) a::b\n";

    // When / Then
    assert_eq!(failure_lines(output), [] as [&str; 0]);
}
