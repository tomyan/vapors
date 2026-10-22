//! Compact output: one line per fact, the failure lines, the fix and
//! the command that reproduces it. Full output only with `--verbose`.

use std::cell::RefCell;
use std::time::Duration;

use crate::command::Outcome;
use crate::steps::Step;
use crate::stream::StreamFilter;

/// Prints step results in the factory's one-line format.
pub struct Reporter {
    verbose: bool,
    stream: RefCell<StreamFilter>,
}

impl Reporter {
    /// A reporter that prints full output when `verbose` is true.
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            stream: RefCell::new(StreamFilter::default()),
        }
    }

    /// Report one step's outcome.
    pub fn step(&self, step: &Step, outcome: &Outcome) {
        let status = if outcome.passed { "ok  " } else { "FAIL" };
        println!("{status} {:<8} {}", step.name, seconds(outcome.duration));
        if self.verbose {
            print!("{}", outcome.output);
        } else if !outcome.passed {
            for line in lines_to_show(&outcome.output) {
                println!("     {line}");
            }
        }
        if !outcome.passed {
            println!("     fix: {}", step.fix);
            println!("     reproduce: {}", step.reproduce());
        }
    }

    /// One line from a long-running tool, as `trunk    <line>`: every
    /// line with `--verbose`, otherwise what [`StreamFilter`] keeps.
    pub fn stream(&self, tool: &str, line: &str) {
        if let Some(text) = self.stream.borrow_mut().line(self.verbose, line) {
            println!("{tool:<8} {text}");
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

/// What to print for a failing step: its classified lines (diagnostics,
/// failures, panics) when there are any; otherwise every non-empty
/// line, because a failing step's cause must never be hidden.
pub fn lines_to_show(output: &str) -> Vec<&str> {
    let classified = failure_lines(output);
    if !classified.is_empty() {
        return classified;
    }
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
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
#[path = "report_tests.rs"]
mod tests;
