//! The scripted driver of the 60 s run (docs/plans/one-scene.md, 3.2):
//! deterministic, from the race's state and the frame's index alone.
//! The gas held, steering toward a point ahead on the centre line,
//! braking into the bends; the camera set to Follow, then Fixed, in
//! turn every 10 s of the frame clock; a livery change at 30 s; a
//! restart after three laps.

use std::f32::consts::{PI, TAU};
use std::time::Duration;

use vapors_racer::model::{Controls, Race};

use crate::circuit::{BEND_STARTS, LAP, distance_of, point_at};
use crate::sampler::INTERVAL;

/// The run's length on the frame clock.
const RUN_LENGTH: Duration = Duration::from_secs(60);

/// The run's frames: [`RUN_LENGTH`] at the frame clock's step, 3,750.
#[allow(clippy::cast_possible_truncation)] // 60 s / 16 ms, checked below
pub const RUN_FRAMES: usize = (RUN_LENGTH.as_millis() / INTERVAL.as_millis()) as usize;
const _: () = assert!(
    RUN_LENGTH.as_millis().is_multiple_of(INTERVAL.as_millis()),
    "the run's length must be a whole number of frames"
);

/// How far ahead on the centre line the driver steers toward.
const LOOK_AHEAD: f32 = 60.0;

/// Angles to the target within this, in radians, steer straight.
const STRAIGHT_AHEAD: f32 = 0.02;

/// Faster than this before a bend, the driver brakes. Not needed to
/// hold the bends (a full turn holds 247 units per second on a bend of
/// radius 150), but it puts the brake into every lap.
const BEND_SPEED: f32 = 190.0;

/// How far before a bend's start the driver brakes.
const BRAKE_ZONE: f32 = 50.0;

/// How often the camera switches, and when the livery changes.
const CAMERA_EVERY: Duration = Duration::from_secs(10);
const LIVERY_AT: Duration = Duration::from_secs(30);

/// A change the script makes at a moment of the run, beside driving.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scripted {
    /// The camera set to Follow (`true`) or Fixed. Named, not toggled:
    /// a restart puts the racer's camera back to Fixed.
    Camera {
        /// Whether the camera follows the car.
        follow: bool,
    },
    /// The craft's livery changes.
    Livery,
}

/// The changes due at frame `frame`: each moment that falls after the
/// frame before's time and at or before this one's.
#[must_use]
pub fn scripted(frame: usize) -> Vec<Scripted> {
    let Some(before) = frame.checked_sub(1) else {
        return Vec::new();
    };
    let (from, to) = (frame_time(before), frame_time(frame));
    let mut due = Vec::new();
    let mark = |time: Duration| time.as_millis() / CAMERA_EVERY.as_millis();
    if mark(from) != mark(to) {
        due.push(Scripted::Camera {
            follow: mark(to) % 2 == 1,
        });
    }
    if from < LIVERY_AT && LIVERY_AT <= to {
        due.push(Scripted::Livery);
    }
    due
}

/// Frame `frame`'s time on the frame clock, from the first frame.
///
/// # Panics
/// `frame` is past `u32::MAX`, 2.2 years of frames: no run reaches it.
#[must_use]
pub fn frame_time(frame: usize) -> Duration {
    let frame =
        u32::try_from(frame).expect("a frame index past u32::MAX: no run has that many frames");
    INTERVAL * frame
}

/// Whether the race has finished and is to start again.
#[must_use]
pub fn restart_due(race: &Race) -> bool {
    race.progress.laps == 3
}

/// The controls for `race`: each full on or off, as keys give them.
#[must_use]
pub fn controls(race: &Race) -> Controls {
    let braking = brakes(race);
    Controls {
        throttle: if braking { 0.0 } else { 1.0 },
        brake: if braking { 1.0 } else { 0.0 },
        steer: steer(race),
    }
}

/// Right (+1) when the target ahead is clockwise of the heading, left
/// (-1) when anticlockwise, else straight.
fn steer(race: &Race) -> f32 {
    let [x, y] = race.position;
    let [tx, ty] = point_at(distance_of(race.position) + LOOK_AHEAD);
    let turn = ((ty - y).atan2(tx - x) - race.heading + PI).rem_euclid(TAU) - PI;
    if turn > STRAIGHT_AHEAD {
        1.0
    } else if turn < -STRAIGHT_AHEAD {
        -1.0
    } else {
        0.0
    }
}

fn brakes(race: &Race) -> bool {
    let distance = distance_of(race.position);
    race.speed > BEND_SPEED
        && BEND_STARTS
            .iter()
            .any(|start| (start - distance).rem_euclid(LAP) < BRAKE_ZONE)
}

#[cfg(test)]
#[path = "driver_tests.rs"]
mod tests;
