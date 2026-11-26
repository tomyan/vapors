//! Live on: enough ticks a period apart (one after the stall, no burst),
//! each shown by a frame soon after it, and no frame that shows nothing
//! new, which a redraw timed by winit or anything else would draw.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use crate::check::Window;
use crate::record::Record;
use crate::script::STALL;

/// The fewest ticks with Live on. On lasts about 1.35 s, which loses
/// three ticks to the stall: nine were seen in six runs (2026-11-22) and
/// ten in three (2026-11-26); seven leaves room for two late ones.
const MIN_TICKS: usize = 7;

/// The shortest and longest gap between ticks, but for the one across the
/// stall. Six runs measured every gap, and the first after Live went on,
/// at 96.3 to 107.2 ms (Xvfb, llvmpipe, 2026-11-22): 20 ms either side.
const GAP_LOW: Duration = Duration::from_millis(80);
const GAP_HIGH: Duration = Duration::from_millis(120);

/// The longest a tick may wait for its frame. Six runs measured at most
/// 6.8 ms (Xvfb, llvmpipe, 2026-11-22); a redraw on a winit timer would
/// wait up to a whole tick.
const LATENCY_LIMIT: Duration = Duration::from_millis(50);

pub fn problems(record: &Record, window: &Window) -> Vec<String> {
    let ticks: Vec<Instant> = record.ticks.iter().map(|(at, _)| *at).collect();
    let mut found = Vec::new();
    if ticks.len() < MIN_TICKS {
        found.push(format!(
            "{} ticks with Live on, fewer than {MIN_TICKS}; fix: the interval",
            ticks.len()
        ));
    }
    found.extend(period(&ticks, window));
    found.extend(unshown(record));
    found.extend(unexplained(record));
    found.extend(latency(record));
    found
}

/// The first tick a period after Live went on; then a period apart, but
/// for the gap across the stall.
fn period(ticks: &[Instant], window: &Window) -> Option<String> {
    let stalled = |gap: &[Instant]| gap[0] < window.stall + STALL && gap[1] > window.stall;
    let first = ticks.first().map(|tick| *tick - window.on_at);
    let gaps = ticks
        .windows(2)
        .filter(|gap| !stalled(gap))
        .map(|gap| gap[1] - gap[0]);
    let wrong: Vec<Duration> = first
        .into_iter()
        .chain(gaps)
        .filter(|gap| *gap < GAP_LOW || *gap > GAP_HIGH)
        .collect();
    (!wrong.is_empty()).then(|| {
        format!("{} tick gaps outside {:?} to {:?}: {wrong:?}; fix: the interval's period and missed-tick behaviour", wrong.len(), GAP_LOW, GAP_HIGH)
    })
}

/// Ticks no frame showed.
fn unshown(record: &Record) -> Option<String> {
    let shown: BTreeSet<u64> = record.frames.iter().map(|frame| frame.shown).collect();
    let missing: Vec<u64> = (1..=record.ticks.len() as u64)
        .filter(|tick| !shown.contains(tick))
        .collect();
    (!missing.is_empty())
        .then(|| format!("ticks {missing:?} were never shown; fix: the frame wake per tick"))
}

/// Frames of the window's own (not serving a request) after the first
/// that show the same tick as the one before.
fn unexplained(record: &Record) -> Option<String> {
    let own: Vec<u64> = record
        .frames
        .iter()
        .skip(1)
        .filter(|frame| !frame.request)
        .map(|frame| frame.shown)
        .collect();
    let repeats = own.windows(2).filter(|pair| pair[0] == pair[1]).count();
    (repeats > 0)
        .then(|| format!("{repeats} frames showed nothing new; fix: what else asks for a frame"))
}

/// Each tick's time to the first frame showing it.
fn latency(record: &Record) -> Option<String> {
    let late: Vec<Duration> = crate::check::latencies(record)
        .into_iter()
        .filter(|wait| *wait > LATENCY_LIMIT)
        .collect();
    (!late.is_empty()).then(|| {
        format!(
            "{} ticks waited over {LATENCY_LIMIT:?} for a frame: {late:?}; fix: the frame wake",
            late.len()
        )
    })
}
