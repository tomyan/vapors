//! The racer's circuit as its centre line, by distance from the start
//! line in the direction of the race (clockwise on screen, Y down): the
//! top straight from the start at (0, -RADIUS), the right bend about
//! (STRAIGHT, 0), the bottom straight, the left bend about (-STRAIGHT, 0).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use vapors_racer::model::{RADIUS, STRAIGHT};

/// One bend's length: half a circle.
const BEND: f32 = PI * RADIUS;

/// One lap's length along the centre line.
pub const LAP: f32 = 4.0 * STRAIGHT + 2.0 * BEND;

/// Where the right bend, the bottom straight, the left bend and the
/// top straight's second half start, by distance.
const SECTIONS: [f32; 4] = [
    STRAIGHT,
    STRAIGHT + BEND,
    3.0 * STRAIGHT + BEND,
    3.0 * STRAIGHT + 2.0 * BEND,
];

/// Where each bend starts, by distance.
pub const BEND_STARTS: [f32; 2] = [SECTIONS[0], SECTIONS[2]];

/// The point on the centre line `distance` from the start, any number
/// of laps on.
#[must_use]
pub fn point_at(distance: f32) -> [f32; 2] {
    let s = distance.rem_euclid(LAP);
    let [right, bottom, left, top] = SECTIONS;
    if s < right {
        [s, -RADIUS]
    } else if s < bottom {
        on_bend(STRAIGHT, -FRAC_PI_2 + (s - right) / RADIUS)
    } else if s < left {
        [STRAIGHT - (s - bottom), RADIUS]
    } else if s < top {
        on_bend(-STRAIGHT, FRAC_PI_2 + (s - left) / RADIUS)
    } else {
        [-STRAIGHT + (s - top), -RADIUS]
    }
}

/// The distance from the start of the point on the centre line nearest
/// `position`.
#[must_use]
pub fn distance_of([x, y]: [f32; 2]) -> f32 {
    let [right, bottom, left, _] = SECTIONS;
    if x > STRAIGHT {
        right + RADIUS * (y.atan2(x - STRAIGHT) + FRAC_PI_2)
    } else if x < -STRAIGHT {
        let angle = y.atan2(x + STRAIGHT).rem_euclid(TAU);
        left + RADIUS * (angle - FRAC_PI_2)
    } else if y > 0.0 {
        bottom + (STRAIGHT - x)
    } else {
        x.rem_euclid(LAP)
    }
}

fn on_bend(centre_x: f32, angle: f32) -> [f32; 2] {
    [centre_x + RADIUS * angle.cos(), RADIUS * angle.sin()]
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod tests;
