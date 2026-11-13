//! Property 6 of `docs/plans/perf-finish-line.md`, measured: an app left
//! alone asks for no frame and does no update. The harness drives the
//! app as the platform would, on the app's clock: a frame that returns
//! `Next::Continue` gets another [`FRAME_INTERVAL`] later (a 60 Hz
//! display), `Next::WaitUntil(at)` one at `at`, `Next::Wait` none, until
//! [`INTERVAL`] of the app's time has passed since it settled. Wall time
//! is not waited on: nothing can wake a headless app (no platform, no
//! waker), a declared limit; what a GPU scope's wake would ask for is
//! not seen here. The frames before the first `Wait` are the settling
//! frames (the mount's), bounded by [`SETTLE_FRAMES`], and not counted.
//! An update counts when its `UpdateCounts` are not all zero. The
//! driver is `sampler_idle.rs`; `idle_run.rs` runs it at each size for
//! `--scaling Idle`.

use std::fmt::Write;
use std::time::{Duration, Instant};

use vapors_elements::UpdateCounts;
use vapors_platform::Next;

/// The app's time an idle run covers after settling.
pub const INTERVAL: Duration = Duration::from_secs(10);

/// Between frames an app asks for at once, as a 60 Hz display's.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// The most frames an app may take to first return `Next::Wait`: the
/// rows page settles in one (the mount's frame); five leaves room for
/// an app whose first update changes the document again.
pub const SETTLE_FRAMES: usize = 5;

/// What an app did when left alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdleCounts {
    /// Frames before the first `Next::Wait`, the mount's.
    pub settled_after: usize,
    /// Frames the app asked for in the interval after settling.
    pub frames: usize,
    /// Of those, frames whose update did work.
    pub updates: usize,
}

/// The platform's next step for what a frame asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Run a frame at this time on the app's clock.
    Frame(Instant),
    /// The interval is over, or the app asked for nothing before it is.
    Done,
}

/// What the platform does after a frame at `now` returned `next`,
/// within an interval ending at `end`.
///
/// # Errors
/// The app exited.
pub fn step(next: Next, now: Instant, end: Instant) -> Result<Step, String> {
    let at = match next {
        Next::Continue => now + FRAME_INTERVAL,
        Next::WaitUntil(at) => at.max(now),
        Next::Wait => return Ok(Step::Done),
        Next::Exit => return Err("the app exited while idle".into()),
    };
    Ok(if at <= end {
        Step::Frame(at)
    } else {
        Step::Done
    })
}

/// Whether an update did work: any of its counts is not zero.
#[must_use]
pub fn did_update(counts: UpdateCounts) -> bool {
    counts != UpdateCounts::default()
}

/// Whether `next` is a settled answer: no frame until something changes.
#[must_use]
pub fn is_settled(next: Next) -> bool {
    matches!(next, Next::Wait | Next::WaitUntil(_))
}

/// The counts at each size as Markdown.
#[must_use]
pub fn section(interval: Duration, measured: &[(usize, IdleCounts)]) -> String {
    let mut out = format!(
        "### Idle, {} s with no input (frames on the app's clock, {} ms apart)\n\n| |",
        interval.as_secs(),
        FRAME_INTERVAL.as_millis()
    );
    for (rows, _) in measured {
        let _ = write!(out, " {rows} rows |");
    }
    out.push('\n');
    out.push_str(&"|---".repeat(measured.len() + 1));
    out.push_str("|\n");
    for (name, read) in [
        (
            "frames to settle",
            (|c: &IdleCounts| c.settled_after) as fn(&IdleCounts) -> usize,
        ),
        ("frames requested", |c| c.frames),
        ("updates", |c| c.updates),
    ] {
        let _ = write!(out, "| {name} |");
        for (_, counts) in measured {
            let _ = write!(out, " {} |", read(counts));
        }
        out.push('\n');
    }
    out
}

/// The line a passing run ends with.
///
/// # Errors
/// A size where the app asked for a frame or did an update, each named
/// with its counts, the command that reproduces it and the fix; no
/// sizes at all.
pub fn verdict(interval: Duration, measured: &[(usize, IdleCounts)]) -> Result<String, String> {
    if measured.is_empty() {
        return Err("idle: no size was measured, so nothing can be judged".into());
    }
    let secs = interval.as_secs();
    let failures: Vec<String> = measured
        .iter()
        .filter(|(_, c)| c.frames > 0 || c.updates > 0)
        .map(|(rows, c)| {
            format!(
                "{rows} rows: {} frames requested and {} updates in {secs} s after settling in {} frames",
                c.frames, c.updates, c.settled_after
            )
        })
        .collect();
    if !failures.is_empty() {
        return Err(format!(
            "idle: the app does work with no input:\n- {}\nreproduce: vapors-perf 60 10 --scaling \
             Idle; the fix is to return Next::Wait from a frame that changed nothing and to skip \
             the update when nothing changed (docs/plans/perf-finish-line.md, property 6)",
            failures.join("\n- ")
        ));
    }
    let sizes: Vec<String> = measured.iter().map(|(rows, _)| rows.to_string()).collect();
    let settled: Vec<String> = measured
        .iter()
        .map(|(_, c)| c.settled_after.to_string())
        .collect();
    Ok(format!(
        "idle: pass, no frames requested and no updates in {secs} s with no input at {} rows \
         (settled after {} frames)",
        sizes.join(", "),
        settled.join(", ")
    ))
}

#[cfg(test)]
#[path = "idle_tests.rs"]
mod tests;
