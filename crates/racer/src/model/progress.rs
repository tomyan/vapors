//! Signed angular progress prevents start-line farming and infield shortcuts.
use super::{ROAD, road_distance};
use std::f32::consts::{FRAC_PI_2, TAU};
/// Progress around the circuit, counted at the start line.
#[derive(Clone, Debug)]
pub struct Progress {
    /// Laps completed, stopping at three.
    pub laps: u8,
    /// Best completed lap in seconds.
    pub best: Option<f32>,
    /// Last completed lap in seconds.
    pub last: Option<f32>,
    previous: f32,
    distance: f32,
    lap_started: f32,
}
impl Default for Progress {
    fn default() -> Self {
        Self {
            laps: 0,
            best: None,
            last: None,
            previous: 0.0,
            distance: 0.0,
            lap_started: 0.0,
        }
    }
}
impl Progress {
    /// Observe the car along the road. Leaving it loses progress to the
    /// next lap: returning at the start restores the full-lap requirement.
    pub fn observe(&mut self, position: [f32; 2], elapsed: f32) {
        let angle = (position[1].atan2(position[0]) + FRAC_PI_2).rem_euclid(TAU);
        let delta =
            (angle - self.previous + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
        self.previous = angle;
        if road_distance(position) > ROAD + 6.0 || delta.abs() > 0.2 {
            self.distance = 0.0;
            return;
        }
        self.distance = (self.distance + delta).max(0.0);
        if self.distance >= TAU - 0.001 && angle < 0.15 && self.laps < 3 {
            self.complete(elapsed);
        }
    }
    fn complete(&mut self, elapsed: f32) {
        let lap = elapsed - self.lap_started;
        self.last = Some(lap);
        self.best = Some(self.best.map_or(lap, |best| best.min(lap)));
        self.lap_started = elapsed;
        self.laps += 1;
        self.distance = 0.0;
    }
}
