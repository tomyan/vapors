//! Compact output: one line per fact, the failure lines, the fix and
//! the command that reproduces it. Full output only with `--verbose`.

use std::time::Duration;

use crate::command::Outcome;
use crate::steps::Step;

/// Prints step results in the factory's one-line format.
pub struct Reporter {
    verbose: bool,
}

impl Reporter {
    /// A reporter that prints full output when `verbose` is true.
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Report one step's outcome.
    pub fn step(&self, step: &Step, outcome: &Outcome) {
        let status = if outcome.passed { "ok  " } else { "FAIL" };
        println!("{status} {:<8} {}", step.name, seconds(outcome.duration));
        if self.verbose {
            print!("{}", outcome.output);
        } else if !outcome.passed {
            for line in failure_lines(&outcome.output) {
                println!("     {line}");
            }
        }
        if !outcome.passed {
            println!("     fix: {}", step.fix);
            println!("     reproduce: {}", step.reproduce());
        }
    }

    /// One line from a long-running tool, as `trunk    <line>`: every
    /// line with `--verbose`, otherwise the ones that mark a rebuild
    /// (starting, success, error, warning) with their timestamp cut.
    pub fn stream(&self, tool: &str, line: &str) {
        if let Some(text) = streamed_line(self.verbose, line) {
            println!("{tool:<8} {text}");
        }
    }

    /// Final line: how many steps passed and the total time.
    pub fn summary(passed: usize, total: usize, elapsed: Duration) {
        let verdict = if passed == total { "ok" } else { "FAIL" };
        println!("{verdict}   {passed}/{total} steps in {}", seconds(elapsed));
    }
}

/// What `stream` prints for `line`, if anything: every line with
/// `verbose`, otherwise only a rebuild line, both without the timestamp.
pub fn streamed_line(verbose: bool, line: &str) -> Option<&str> {
    let text = without_timestamp(line);
    (verbose || is_rebuild_line(text)).then_some(text)
}

/// trunk prefixes each line with an RFC 3339 timestamp and a level;
/// the text after the level is what a reader needs.
fn without_timestamp(line: &str) -> &str {
    let trimmed = line.trim();
    match trimmed.split_once("  ") {
        Some((stamp, rest)) if stamp.ends_with('Z') && stamp.contains('T') => rest.trim(),
        _ => trimmed,
    }
}

fn is_rebuild_line(text: &str) -> bool {
    [
        "starting build",
        "success",
        "error",
        "warning",
        "server listening",
        "http://",
    ]
    .iter()
    .any(|mark| text.contains(mark))
}

fn seconds(duration: Duration) -> String {
    format!("{:.1}s", duration.as_secs_f64())
}

/// The lines of tool output worth reading when a step fails: rustc and
/// clippy diagnostics with their locations, nextest failures, and panics
/// with their message (the lines after `panicked at`, up to a blank line
/// or a `note:`). Trimmed, and with repeats removed, because nextest
/// prints each failure twice.
pub fn failure_lines(output: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = Vec::new();
    let mut in_panic_message = false;
    for line in output.lines().map(str::trim) {
        in_panic_message = continues_panic_message(in_panic_message, line);
        if (in_panic_message || is_failure_line(line)) && !lines.contains(&line) {
            lines.push(line);
        }
        if line.contains("panicked at") {
            in_panic_message = true;
        }
    }
    lines
}

fn continues_panic_message(was_in_message: bool, line: &str) -> bool {
    was_in_message && !line.is_empty() && !line.starts_with("note:")
}

fn is_failure_line(line: &str) -> bool {
    line.starts_with("error")
        || line.starts_with("warning:")
        || line.starts_with("-->")
        || line.starts_with("FAIL [")
        || line.contains("panicked at")
        || line.starts_with("Diff in")
        || line.starts_with("could not start")
}

#[cfg(test)]
mod tests {
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
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    #[test]
    fn trunks_timestamp_and_level_are_cut_from_a_line() {
        // When / Then
        assert_eq!(
            without_timestamp("2026-10-22T10:28:39.440574Z  INFO ✅ success"),
            "INFO ✅ success"
        );
        assert_eq!(
            without_timestamp("   Compiling wgpu v30.0.1"),
            "Compiling wgpu v30.0.1"
        );
    }

    #[test]
    fn stream_prints_every_line_with_verbose_and_rebuild_lines_without() {
        // Given
        let compiling = "   Compiling wgpu v30.0.1";
        let success = "2026-10-22T10:28:39.440574Z  INFO ✅ success";

        // When / Then
        assert_eq!(streamed_line(false, compiling), None);
        assert_eq!(streamed_line(false, success), Some("INFO ✅ success"));
        assert_eq!(
            streamed_line(true, compiling),
            Some("Compiling wgpu v30.0.1")
        );
    }

    #[test]
    fn only_rebuild_lines_are_streamed_without_verbose() {
        // When / Then
        assert!(is_rebuild_line("INFO 📦 starting build"));
        assert!(is_rebuild_line("INFO ✅ success"));
        assert!(is_rebuild_line("ERROR error from build pipeline"));
        assert!(is_rebuild_line("INFO 📡 server listening at:"));
        assert!(!is_rebuild_line("Compiling wgpu v30.0.1"));
        assert!(!is_rebuild_line("INFO node packages []"));
    }
}
