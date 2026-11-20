//! The live switch of the top bar: while it is on, a tick every
//! `LIVE_INTERVAL_MS` and a spinner turning once every `SPINNER_PERIOD_MS`,
//! both read from the frame clock (`vapors_elements::frame_time`), which
//! keeps the loop drawing only while one of them is read.

use crate::dashboard::{LIVE_INTERVAL_MS, SPINNER_PERIOD_MS, SPINNER_RADIUS, SPINNER_SIZE};
use std::{cell::Cell, f32::consts::TAU, fmt::Write, time::Duration};
use vapors_elements::frame_time;
use vapors_signals::{ReadSignal, Signal};

/// The state of the live switch.
#[derive(Clone, Copy)]
pub struct Live {
    /// Whether the switch is on; off at startup.
    pub on: Signal<bool>,
    /// The ticks since the switch went on: 0 while off.
    pub tick: ReadSignal<u64>,
    /// The `d` of the spinner's path, turned to the clock; empty while off.
    pub spinner: ReadSignal<String>,
}

impl Live {
    /// The startup state: off.
    pub fn new() -> Self {
        let on = Signal::named("live", false);
        let phase = Cell::new(Phase::Off);
        let tick = Signal::derived(move || {
            if !on.get() {
                phase.set(Phase::Off);
                return 0;
            }
            let (next, tick) = phase.get().step(frame_time().get());
            phase.set(next);
            tick
        });
        let spinner = Signal::derived(move || {
            if on.get() {
                spinner_path(frame_time().get())
            } else {
                String::new()
            }
        });
        Live { on, tick, spinner }
    }
}

/// Where the tick counter is. The switch is clicked between two frames, so
/// the clock it first reads is the last frame's, older by the whole idle
/// wait: the count starts at the second time it sees, which is a frame's.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Off,
    Seen(Duration),
    Running(Duration),
}

impl Phase {
    /// The phase and the tick after reading the clock at `now`.
    pub fn step(self, now: Duration) -> (Phase, u64) {
        match self {
            Phase::Off => (Phase::Seen(now), 0),
            Phase::Seen(at) if at == now => (self, 0),
            Phase::Seen(_) => (Phase::Running(now), 0),
            Phase::Running(base) => {
                let ticks = (now.saturating_sub(base).as_millis() / u128::from(LIVE_INTERVAL_MS))
                    .try_into()
                    .expect("a tick count fits u64");
                (self, ticks)
            }
        }
    }
}

/// The spinner's path at `time`: the spec's square, its corners rounded by
/// `SPINNER_RADIUS`, turned clockwise about its centre at a constant speed.
/// vapors has no `transform` (FLAGS.md), so the corners are turned here.
#[allow(clippy::cast_precision_loss)] // a period's milliseconds are small
pub fn spinner_path(time: Duration) -> String {
    let turn = (time.as_millis() % u128::from(SPINNER_PERIOD_MS)) as f32 / SPINNER_PERIOD_MS as f32;
    let (sin, cos) = (turn * TAU).sin_cos();
    let (half, r) = (SPINNER_SIZE / 2.0, SPINNER_RADIUS);
    let at = |x: f32, y: f32| {
        format!(
            "{:.3} {:.3}",
            half + x * cos - y * sin,
            half + x * sin + y * cos
        )
    };
    // each corner of the square, clockwise from the top right: its centre
    // and the directions of its arc's first and last point
    let corners = [
        (1.0, -1.0, (0.0, -1.0), (1.0, 0.0)),
        (1.0, 1.0, (1.0, 0.0), (0.0, 1.0)),
        (-1.0, 1.0, (0.0, 1.0), (-1.0, 0.0)),
        (-1.0, -1.0, (-1.0, 0.0), (0.0, -1.0)),
    ];
    let mut path = format!("M{}", at(-half + r, -half));
    for (sx, sy, from, to) in corners {
        let (cx, cy) = (sx * (half - r), sy * (half - r));
        let (start, end) = (
            at(cx + from.0 * r, cy + from.1 * r),
            at(cx + to.0 * r, cy + to.1 * r),
        );
        write!(path, " L{start} A{r} {r} 0 0 1 {end}").expect("a String takes text");
    }
    path + " Z"
}
