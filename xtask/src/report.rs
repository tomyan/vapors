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

    /// Final line: how many steps passed and the total time.
    pub fn summary(passed: usize, total: usize, elapsed: Duration) {
        let verdict = if passed == total { "ok" } else { "FAIL" };
        println!("{verdict}   {passed}/{total} steps in {}", seconds(elapsed));
    }
}

fn seconds(duration: Duration) -> String {
    format!("{:.1}s", duration.as_secs_f64())
}

/// The lines of tool output worth reading when a step fails: rustc and
/// clippy diagnostics with their locations, nextest failures and panics.
/// Trimmed, and with repeats removed (nextest prints each failure twice).
pub fn failure_lines(output: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = Vec::new();
    for line in output.lines().map(str::trim) {
        if is_failure_line(line) && !lines.contains(&line) {
            lines.push(line);
        }
    }
    lines
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
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("FAIL ["));
        assert!(lines[1].contains("panicked at crates/scene/src/color.rs:40:9"));
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
