//! The judgement of `--check`: every rule the record breaks, by name.
//! Empty when the run is right.

use std::thread::ThreadId;
use std::time::{Duration, Instant};

use crate::TICK;
use crate::record::{Phase, Record};

/// The fewest ticks the `On` phase must show.
const MIN_TICKS: usize = 5;

/// Every broken rule, given the application thread's id.
pub fn problems(record: &Record, app: ThreadId) -> Vec<String> {
    let mut found = Vec::new();
    threads(record, app, &mut found);
    let Some(phases) = Phases::of(record) else {
        found.push(format!(
            "the script's phases are incomplete: {:?}",
            record.phases
        ));
        return found;
    };
    idle(record, &phases, &mut found);
    ticking(record, &phases, &mut found);
    found
}

fn threads(record: &Record, app: ThreadId, found: &mut Vec<String>) {
    match record.ui_thread {
        None => found.push("no frame recorded the UI thread".to_owned()),
        Some(ui) if ui == app => found.push("the UI and the application thread are one".to_owned()),
        Some(_) => {}
    }
    let off = |ids: &mut dyn Iterator<Item = ThreadId>| ids.filter(|id| *id != app).count();
    let handlers = off(&mut record.handlers.iter().copied());
    if record.handlers.is_empty() || handlers > 0 {
        found.push(format!(
            "{handlers} of {} handlers ran off the application thread",
            record.handlers.len()
        ));
    }
    let ticks = off(&mut record.ticks.iter().map(|(_, id)| *id));
    if ticks > 0 {
        found.push(format!(
            "{ticks} of {} tick continuations ran off the application thread",
            record.ticks.len()
        ));
    }
}

pub struct Phases {
    pub on: Instant,
    pub stall: Instant,
    pub off_again: Instant,
    pub end: Instant,
    pub off: Instant,
}

impl Phases {
    fn of(record: &Record) -> Option<Phases> {
        Some(Phases {
            off: record.at(Phase::Off)?,
            on: record.at(Phase::On)?,
            stall: record.at(Phase::Stall)?,
            off_again: record.at(Phase::OffAgain)?,
            end: record.at(Phase::End)?,
        })
    }
}

fn between(times: impl Iterator<Item = Instant>, from: Instant, to: Instant) -> usize {
    times.filter(|at| *at > from && *at < to).count()
}

/// Live off: no frames and no ticks, after one tick's grace for a tick
/// already on its way when Live went off.
fn idle(record: &Record, phases: &Phases, found: &mut Vec<String>) {
    let frames = record.frames.iter().copied();
    let off_frames = between(frames.clone(), phases.off, phases.on);
    let settled = phases.off_again + TICK;
    let again_frames = between(frames, settled, phases.end);
    let again_ticks = between(
        record.ticks.iter().map(|(at, _)| *at),
        phases.off_again,
        phases.end,
    );
    if off_frames + again_frames + again_ticks > 0 {
        found.push(format!(
            "with Live off: {off_frames} frames before it went on, {again_frames} frames and \
             {again_ticks} ticks after it went off"
        ));
    }
}

/// Live on: enough ticks, a frame for each within one, and no burst
/// after the stall.
fn ticking(record: &Record, phases: &Phases, found: &mut Vec<String>) {
    let ticks: Vec<Instant> = record.ticks.iter().map(|(at, _)| *at).collect();
    let on_ticks = between(ticks.iter().copied(), phases.on, phases.off_again);
    let on_frames = between(
        record.frames.iter().copied(),
        phases.on,
        phases.off_again + TICK,
    );
    if on_ticks < MIN_TICKS {
        found.push(format!(
            "{on_ticks} ticks with Live on, fewer than {MIN_TICKS}"
        ));
    }
    if on_frames.abs_diff(on_ticks) > 1 {
        found.push(format!(
            "{on_frames} frames for {on_ticks} ticks with Live on"
        ));
    }
    let burst = ticks
        .windows(2)
        .filter(|w| w[0] > phases.stall && w[1] - w[0] < TICK / 2)
        .count();
    if burst > 0 {
        found.push(format!(
            "{burst} ticks less than {:?} after the one before, after the stall",
            TICK / 2
        ));
    }
}

/// The time from each tick to the first frame after it.
pub fn latencies(record: &Record) -> Vec<Duration> {
    let mut found: Vec<Duration> = record
        .ticks
        .iter()
        .filter_map(|(tick, _)| {
            record
                .frames
                .iter()
                .find(|frame| *frame >= tick)
                .map(|f| *f - *tick)
        })
        .collect();
    found.sort();
    found
}
