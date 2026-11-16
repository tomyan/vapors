//! A `--first-frame` page as text: one summary line, then a table of
//! each part's phases. Every line has one of the shapes
//! `scripts/first-frame-table.sh` reads, which refuses any other.

use std::fmt::Write;

use crate::first_frame_run::micros;
use crate::mount::Mounted;
use crate::sampler::{FirstFrame, Phases};

/// The page of `rows` rows: its mount `mounted`, then `first`.
#[must_use]
pub fn report(rows: usize, mounted: Mounted, first: &FirstFrame) -> String {
    let total = mounted.time + first.start + first.frame_idle;
    let mut out = format!(
        "First frame, {rows} rows: mount {} µs, App::start to idle {} µs, frame 0 to idle {} µs \
         (to submit {} µs); mount to frame 0 idle {} µs.\n\n",
        micros(mounted.time),
        micros(first.start),
        micros(first.frame_idle),
        micros(first.frame_cpu),
        micros(total)
    );
    out.push_str("| part | phase | µs | instructions |\n|---|---|---|---|\n");
    row(
        &mut out,
        ("mount", "all"),
        &micros(mounted.time),
        Some(mounted.instructions),
    );
    row(&mut out, ("start", "all"), &micros(first.start), None);
    phases(&mut out, "start", &first.start_phases);
    row(
        &mut out,
        ("frame", "all"),
        &micros(first.frame_cpu),
        first.frame_instructions,
    );
    row(
        &mut out,
        ("frame", "to idle"),
        &micros(first.frame_idle),
        None,
    );
    phases(&mut out, "frame", &first.frame_phases);
    out
}

fn phases(out: &mut String, part: &str, (times, counts): &Phases) {
    for (phase, time) in times {
        let count = counts
            .as_ref()
            .and_then(|counts| counts.get(phase))
            .copied();
        row(out, (part, phase), &micros(*time), count);
    }
}

fn row(out: &mut String, (part, phase): (&str, &str), time: &str, count: Option<u64>) {
    let count = count.map_or_else(|| "none".to_owned(), |count| count.to_string());
    let _ = writeln!(out, "| {part} | {phase} | {time} | {count} |");
}

#[cfg(test)]
#[path = "first_frame_report_tests.rs"]
mod tests;
