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
/// clippy diagnostics with their locations, nextest failures, signals
/// and timeouts, panics with their message (the lines after `panicked
/// at`, up to a blank line or a `note:`), and cargo's `Caused by:`
/// blocks whole, which hold a failed build script's `--- stdout` and
/// `--- stderr`. Trimmed, and with repeats removed, because nextest
/// prints each failure twice.
pub fn failure_lines(output: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = Vec::new();
    let mut in_panic_message = false;
    let mut in_cause = false;
    for raw in output.lines() {
        let line = raw.trim();
        in_panic_message = continues_panic_message(in_panic_message, line);
        in_cause = continues_cause(in_cause, raw);
        let kept = in_panic_message || (in_cause && !line.is_empty()) || is_failure_line(line);
        if kept && !lines.contains(&line) {
            lines.push(line);
        }
        in_panic_message |= line.contains("panicked at");
        in_cause |= line == "Caused by:";
    }
    lines
}

fn continues_panic_message(was_in_message: bool, line: &str) -> bool {
    was_in_message && !line.is_empty() && !line.starts_with("note:")
}

/// Cargo indents everything under `Caused by:`, with blank lines
/// between a build script's stdout and stderr; the block ends at the
/// next unindented line.
fn continues_cause(was_in_cause: bool, raw: &str) -> bool {
    was_in_cause && (raw.trim().is_empty() || raw.starts_with(char::is_whitespace))
}

fn is_failure_line(line: &str) -> bool {
    line.starts_with("error")
        || line.starts_with("warning:")
        || line.starts_with("-->")
        || line.starts_with("FAIL [")
        || is_signal_or_timeout(line)
        || line.contains("panicked at")
        || line == "Caused by:"
        || line.starts_with("Diff in")
        || line.starts_with("could not start")
        // cargo-deny's advisory: which one, and its fix.
        || line.starts_with("├ ID:")
        || line.starts_with("├ Solution:")
}

/// nextest's line for a test killed by a signal (`SIGSEGV [ 0.01s]`,
/// `SIGABRT [`, any `SIG` name) or by its deadline (`TIMEOUT [`).
fn is_signal_or_timeout(line: &str) -> bool {
    let Some((word, rest)) = line.split_once(' ') else {
        return false;
    };
    let signal = word.starts_with("SIG")
        && word
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    (signal || word == "TIMEOUT") && rest.trim_start().starts_with('[')
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "report_causes_tests.rs"]
mod causes_tests;
