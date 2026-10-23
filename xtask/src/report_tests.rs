//! What the reporter shows for a failing step and what it streams.

use super::*;

#[test]
fn clippy_diagnostic_keeps_message_and_location() {
    // Given
    let output = "    Checking scene\nwarning: unused variable: `x`\n  --> crates/scene/src/lib.rs:3:9\n   |\n3  |     let x = 1;\n";

    // When
    let lines = failure_lines(output);

    // Then
    assert_eq!(
        lines,
        [
            "warning: unused variable: `x`",
            "--> crates/scene/src/lib.rs:3:9"
        ]
    );
}

#[test]
fn nextest_failure_keeps_test_name_and_panic() {
    // Given
    let output = "        PASS [   0.01s] xtask steps::tests::ci_runs_fastest_checks_first\n        FAIL [   0.01s] vapors-scene color::tests::round_trip\nthread 'color::tests::round_trip' panicked at crates/scene/src/color.rs:40:9:\nassertion failed\n";

    // When
    let lines = failure_lines(output);

    // Then
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("FAIL ["));
    assert!(lines[1].contains("panicked at crates/scene/src/color.rs:40:9"));
    assert_eq!(lines[2], "assertion failed");
}

#[test]
fn panic_message_lines_follow_the_panicked_at_line() {
    // Given
    let output = "thread 't' panicked at crates/x.rs:9:5:\nno GPU adapter: wanted \"nvidia\"; found llvmpipe.\nFix: set VAPORS_GPU_ADAPTER.\nnote: run with `RUST_BACKTRACE=1`\n\n   Summary [ 0.1s]\n";

    // When
    let lines = failure_lines(output);

    // Then
    assert_eq!(
        lines,
        [
            "thread 't' panicked at crates/x.rs:9:5:",
            "no GPU adapter: wanted \"nvidia\"; found llvmpipe.",
            "Fix: set VAPORS_GPU_ADAPTER."
        ]
    );
}

#[test]
fn repeated_failure_lines_appear_once() {
    // Given
    let output = "  FAIL [ 0.1s] a::b
summary
  FAIL [ 0.1s] a::b
";

    // When
    let lines = failure_lines(output);

    // Then
    assert_eq!(lines, ["FAIL [ 0.1s] a::b"]);
}

#[test]
fn passing_output_has_no_failure_lines() {
    // Given
    let output = "    Finished `dev` profile\n        PASS [   0.01s] xtask steps::tests::x\n";

    // When
    let lines = failure_lines(output);

    // Then
    assert_eq!(lines, [] as [&str; 0]);
}

#[test]
fn classified_lines_are_shown_alone_when_there_are_any() {
    // Given
    let output =
        "   Compiling x\nerror[E0425]: cannot find value\n --> src/a.rs:1:1\nsome chatter\n";

    // When / Then
    assert_eq!(
        lines_to_show(output),
        ["error[E0425]: cannot find value", "--> src/a.rs:1:1"]
    );
}

#[test]
fn every_line_is_shown_when_none_is_classified() {
    // Given: a tool that failed without a diagnostic shape
    let output = ".config/factory.toml:0: could not read it: No such file\n\n  run from the workspace root\n";

    // When / Then
    assert_eq!(
        lines_to_show(output),
        [
            ".config/factory.toml:0: could not read it: No such file",
            "run from the workspace root"
        ]
    );
}

#[test]
fn a_cargo_deny_advisory_shows_its_id_and_solution() {
    // Given: cargo-deny's report of a vulnerable dependency
    let output = "error[vulnerability]: Potential segfault in the time crate\n   ┌─ /w/Cargo.lock:60:1\n   │\n60 │ time 0.1.45 registry+https://github.com/rust-lang/crates.io-index\n   │ ----------------------------------------------------------------- security vulnerability\n   │\n   ├ ID: RUSTSEC-2020-0071\n   ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2020-0071\n   ├ Solution: Upgrade to >=0.2.23 (try `cargo update -p time`)\n";

    // When
    let lines = lines_to_show(output);

    // Then
    assert!(lines.contains(&"├ ID: RUSTSEC-2020-0071"), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("├ Solution:")),
        "{lines:?}"
    );
}
