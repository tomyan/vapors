//! The pre-push hook (hooks/pre-push): a push to main is refused before
//! any check runs, unless `ALLOW_PUSH_TO_MAIN=1`, which it prints.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const HOOK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../hooks/pre-push");

/// A directory holding a `cargo` that succeeds and records that it ran.
fn stub_cargo(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-hook-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("stub dir");
    let cargo = dir.join("cargo");
    std::fs::write(&cargo, format!("#!/bin/sh\ntouch {}/ran\n", dir.display())).expect("stub");
    let made = Command::new("chmod")
        .arg("755")
        .arg(&cargo)
        .status()
        .expect("chmod");
    assert!(made.success());
    dir
}

/// The hook run with git's push lines on stdin.
fn push(test: &str, lines: &str, allow: Option<&str>) -> (Output, bool) {
    let stub = stub_cargo(test);
    let mut command = Command::new("sh");
    command
        .arg(HOOK)
        .args(["origin", "https://example.invalid/repo.git"])
        .env("PATH", format!("{}:/usr/bin:/bin", stub.display()))
        .env_remove("ALLOW_PUSH_TO_MAIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(value) = allow {
        command.env("ALLOW_PUSH_TO_MAIN", value);
    }
    let mut child = command.spawn().expect("the hook");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(lines.as_bytes())
        .expect("lines");
    let output = child.wait_with_output().expect("its exit");
    (output, stub.join("ran").exists())
}

const SHA: &str = "1111111111111111111111111111111111111111";

#[test]
fn a_push_to_main_is_refused_before_any_check_runs() {
    // When
    let (output, ran) = push(
        "main",
        &format!("refs/heads/x {SHA} refs/heads/main {SHA}\n"),
        None,
    );

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("push your branch; main changes only by PR merge"),
        "{stderr}"
    );
    assert!(!ran, "the checks ran before the refusal");
}

#[test]
fn a_push_to_a_branch_runs_the_checks() {
    // When
    let (output, ran) = push(
        "branch",
        &format!("refs/heads/x {SHA} refs/heads/x {SHA}\n"),
        None,
    );

    // Then
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(ran);
}

#[test]
fn the_override_lets_a_push_to_main_through_and_says_so() {
    // When
    let (output, ran) = push(
        "allow",
        &format!("refs/heads/x {SHA} refs/heads/main {SHA}\n"),
        Some("1"),
    );

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("ALLOW_PUSH_TO_MAIN=1"), "{stderr}");
    assert!(ran);
}

const ZERO: &str = "0000000000000000000000000000000000000000";

#[test]
fn a_push_of_several_refs_with_main_among_them_is_refused() {
    // When
    let lines = format!(
        "refs/heads/x {SHA} refs/heads/x {SHA}\nrefs/heads/y {SHA} refs/heads/main {SHA}\n"
    );
    let (output, ran) = push("several", &lines, None);

    // Then
    assert!(!output.status.success());
    assert!(!ran);
}

#[test]
fn deleting_main_is_refused_with_a_reproduce_line_for_a_delete() {
    // When
    let (output, ran) = push(
        "delete",
        &format!("(delete) {ZERO} refs/heads/main {SHA}\n"),
        None,
    );

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("git push origin --delete main"), "{stderr}");
    assert!(!ran);
}

#[test]
fn an_override_other_than_1_does_not_let_a_push_to_main_through() {
    // When
    let (output, ran) = push(
        "yes",
        &format!("refs/heads/x {SHA} refs/heads/main {SHA}\n"),
        Some("yes"),
    );

    // Then
    assert!(!output.status.success());
    assert!(!ran);
}

#[test]
fn empty_stdin_is_refused_naming_it() {
    // When
    let (output, ran) = push("empty", "", None);

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("no push lines on stdin"), "{stderr}");
    assert!(!ran);
}

#[test]
fn a_last_line_without_a_newline_is_still_read() {
    // When
    let (output, ran) = push(
        "no-newline",
        &format!("refs/heads/x {SHA} refs/heads/main {SHA}"),
        None,
    );

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("would update main"),
        "read as no lines: {stderr}"
    );
    assert!(!ran);
}
