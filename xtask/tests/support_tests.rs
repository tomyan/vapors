//! The isolation `support` gives a child xtask (issue #358), and its
//! time limit.

#[allow(
    dead_code,
    reason = "each test binary compiles `support` and uses part of it"
)]
mod support;

use std::process::Command;
use std::time::Duration;

/// Prefixes of variables a child must not see: a toolchain, the test
/// runner's, or the crate under test's.
const FOREIGN: [&str; 6] = [
    "RUSTUP_",
    "NEXTEST",
    "CARGO_MANIFEST",
    "CARGO_TARGET",
    "CARGO_PKG",
    "RUSTFLAGS",
];

#[test]
fn a_child_sees_only_the_stand_ins_and_its_own_variables() {
    // Given
    let scratch = tempfile::Builder::new()
        .prefix("xtask-isolated-")
        .tempdir()
        .expect("scratch");
    let (dir, tmp) = (scratch.path(), scratch.path().join("tmp"));
    std::fs::create_dir(&tmp).expect("tmp");
    let bin = support::bin(dir, support::NO_WORK);

    // When
    let mut child = support::xtask(dir, &bin, &tmp);
    child.arg("test");
    let output = support::output_within(child, Duration::from_secs(5));

    // Then
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("stand-in cargo: nextest run"), "{text}");
    let calls = std::fs::read_to_string(dir.join("sccache.log")).expect("the stand-in sccache ran");
    assert!(calls.contains("--start-server"), "{calls}");
    let seen = std::fs::read_to_string(dir.join("cargo.env")).expect("the stand-in cargo ran");
    let foreign: Vec<&str> = seen
        .lines()
        .filter(|line| FOREIGN.iter().any(|p| line.starts_with(p)))
        .collect();
    assert_eq!(foreign, Vec::<&str>::new());
    assert_eq!(support::entries(&tmp), Vec::<String>::new());
}

#[test]
fn output_within_reads_both_streams_while_the_child_runs() {
    // Given: more than a pipe's 64 KiB on each stream
    let mut child = Command::new("sh");
    child.args([
        "-c",
        "head -c 200000 /dev/zero; head -c 200000 /dev/zero >&2",
    ]);

    // When
    let output = support::output_within(child, Duration::from_secs(5));

    // Then
    assert!(output.status.success());
    assert_eq!(
        (output.stdout.len(), output.stderr.len()),
        (200_000, 200_000)
    );
}

#[test]
#[should_panic(expected = "still running after 200ms, killed; stdout: out; stderr: err")]
fn output_within_kills_a_child_past_its_limit_naming_both_streams() {
    // Given
    let mut child = Command::new("sh");
    child.args(["-c", "echo out; echo err >&2; exec sleep 30"]);

    // When, Then
    support::output_within(child, Duration::from_millis(200));
}

#[test]
fn output_within_keeps_the_child_in_the_test_s_process_group_for_nextest_s_kill() {
    // Given
    let mut child = Command::new("sh");
    child.args(["-c", "ps -o pgid= -p $$; ps -o pgid= -p $PPID"]);

    // When
    let output = support::output_within(child, Duration::from_secs(5));

    // Then
    let text = String::from_utf8_lossy(&output.stdout);
    let groups: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(groups.len(), 2, "{text}");
    assert_eq!(groups[0], groups[1], "the child's group and the test's");
}
