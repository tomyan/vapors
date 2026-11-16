//! Property 6 of `docs/plans/perf-finish-line.md`, measured: an app left
//! alone asks for no frame and does no update. The harness drives the
//! app as the platform would, on the app's clock: a frame that returns
//! `Next::Continue` gets another [`FRAME_INTERVAL`] later (a 60 Hz
//! display), `Next::WaitUntil(at)` one at `at`, `Next::Wait` none, until
//! [`INTERVAL`] of the app's time has passed since it settled. The
//! frames before the first `Wait` or `WaitUntil` are the settling frames
//! (the mount's), bounded by [`SETTLE_FRAMES`]; their updates are not
//! counted. An update counts when its `UpdateCounts` show work
//! ([`did_update`]): `ElementsApp` runs `Document::update` every frame
//! and the update resets them first, so a frame that changed nothing
//! reads its own counts, not the last update's (`gpu_idle_frames.rs`
//! pins both).
//!
//! Declared limit: wall time is not waited on and nothing can wake a
//! headless app (no platform, no waker), so a timer, a GPU scope's wake
//! or a readback that asks for a frame is not seen, and the workload's
//! `change` takes the accessibility update each frame. The verdict line
//! says so. Retired by a waker model in the harness. The driver is
//! `sampler_idle.rs`; `idle_run.rs` runs it at each size for `--scaling
//! Idle`.

use std::fmt::Write;
use std::time::{Duration, Instant};

use vapors_elements::UpdateCounts;
use vapors_platform::Next;

/// The app's time an idle run covers after settling: ten times the
/// longest deadline an idle page can arm, the GPU scope's 1 s
/// (`painter_surface_scope::DEADLINE`; the tooltip's show delay and the
/// wheel latch are 500 ms), so a wake on any of them falls inside it.
pub const INTERVAL: Duration = Duration::from_secs(10);

/// Between frames an app asks for at once, as a 60 Hz display's.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// The most frames an app may take to first return `Wait` or
/// `WaitUntil`: the rows page takes 2 (the mount's frame changes the
/// document, the next draws it; measured 2026-11-13 at every size),
/// and 5 leaves room for one more such round.
pub const SETTLE_FRAMES: usize = 5;

/// The most frames the interval holds: what a 60 Hz display would show.
/// An app asking for more (a deadline in the past, asked again) is
/// refused at this count instead of driven forever.
#[must_use]
pub fn max_frames(interval: Duration) -> usize {
    usize::try_from(interval.as_millis() / FRAME_INTERVAL.as_millis()).unwrap_or(usize::MAX)
}

/// One count of [`IdleCounts`], for the table's rows.
type Read = fn(&IdleCounts) -> usize;

/// What an app did when left alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdleCounts {
    /// Frames before the first `Wait` or `WaitUntil`, the mount's.
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
/// within an interval ending at `end`. A deadline already passed gets
/// its frame at `now`; the driver's cap ([`max_frames`]) bounds an app
/// that keeps asking for one.
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

/// Whether an update did work: it checked, visited or restyled an
/// element, or built, laid out, placed, moved, painted or shifted a
/// box. Not the gauges and call counts an unchanged update still
/// reports (`styles_live`, `layout_calls`; `gpu_idle_frames.rs` pins
/// an unchanged frame at no work).
#[must_use]
pub fn did_update(counts: UpdateCounts) -> bool {
    [
        counts.nodes_checked,
        counts.elements_visited,
        counts.elements_restyled,
        counts.boxes_built,
        counts.boxes_laid_out,
        counts.boxes_placed,
        counts.boxes_moved,
        counts.boxes_painted,
        counts.boxes_shifted,
    ]
    .iter()
    .any(|&count| count > 0)
}

/// Whether `next` is a settled answer: no frame until something changes
/// or a deadline. A deadline beyond the interval asks for nothing in it.
#[must_use]
pub fn is_settled(next: Next) -> bool {
    matches!(next, Next::Wait | Next::WaitUntil(_))
}

/// The counts at each size as Markdown.
#[must_use]
pub fn section(interval: Duration, measured: &[(usize, IdleCounts)]) -> String {
    let mut out = format!(
        "### Idle, {} s with no input after settling (frames on the app's clock, {} ms \
         apart; the settling frames' updates are not counted)\n\n| |",
        interval.as_secs(),
        FRAME_INTERVAL.as_millis()
    );
    for (rows, _) in measured {
        let _ = write!(out, " {rows} rows |");
    }
    out.push('\n');
    out.push_str(&"|---".repeat(measured.len() + 1));
    out.push_str("|\n");
    let rows: [(&str, Read); 3] = [
        ("frames to settle", |c| c.settled_after),
        ("frames requested", |c| c.frames),
        ("updates", |c| c.updates),
    ];
    for (name, read) in rows {
        let _ = write!(out, "| {name} |");
        for (_, counts) in measured {
            let _ = write!(out, " {} |", read(counts));
        }
        out.push('\n');
    }
    out
}

/// The line a passing run ends with, carrying the harness's limit.
///
/// # Errors
/// A size where the app asked for a frame or did an update, each named
/// with its counts, `reproduce` (the run's own command) and the fix; no
/// sizes at all.
pub fn verdict(
    interval: Duration,
    measured: &[(usize, IdleCounts)],
    reproduce: &str,
) -> Result<String, String> {
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
            "idle: the app does work with no input:\n- {}\nreproduce: {reproduce}; the fix is to \
             return Next::Wait from a frame that changed nothing and to skip the update when \
             nothing changed (docs/plans/perf-finish-line.md, property 6)",
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
         (settled after {} frames); headless, so a wake from a timer, a GPU scope or a readback \
         is not seen (retired by a waker model in the harness)",
        sizes.join(", "),
        settled.join(", ")
    ))
}

#[cfg(test)]
#[path = "idle_tests.rs"]
mod tests;
