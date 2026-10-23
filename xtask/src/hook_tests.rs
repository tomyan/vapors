//! The pre-push hook (hooks/pre-push): a push to main is refused before
//! any check runs, unless `ALLOW_PUSH_TO_MAIN=1`, which it prints.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const HOOK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../hooks/pre-push");

/// A directory holding a `cargo` that succeeds and records that it ran.
fn stub_cargo(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-hook-{test}-{}", std::process::id()));
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
fn push(test: &str, lines: &str, allow: bool) -> (Output, bool) {
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
    if allow {
        command.env("ALLOW_PUSH_TO_MAIN", "1");
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
        false,
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
        false,
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
        true,
    );

    // Then
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("ALLOW_PUSH_TO_MAIN=1"), "{stderr}");
    assert!(ran);
}
