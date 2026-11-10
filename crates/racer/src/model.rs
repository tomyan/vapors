//! Deterministic simulation. Distances are world units; time is seconds.
mod camera;
mod progress;
mod settings;
pub use camera::Camera;
pub use progress::Progress;
pub use settings::{Controls, Settings};

/// Half the length of the circuit's straight segment.
pub const STRAIGHT: f32 = 180.0;
/// Radius of each semicircular end.
pub const RADIUS: f32 = 150.0;
/// Half the road width.
pub const ROAD: f32 = 34.0;
/// Maximum speed on tarmac, world units per second.
pub const MAX_SPEED: f32 = 220.0;

/// Distance from the track's centre line.
#[must_use]
pub fn road_distance([x, y]: [f32; 2]) -> f32 {
    ((x - x.clamp(-STRAIGHT, STRAIGHT)).hypot(y) - RADIUS).abs()
}

/// Car, timer and ordered checkpoints for a three-lap run.
#[derive(Clone, Debug, PartialEq)]
pub struct Race {
    /// World position.
    pub position: [f32; 2],
    /// Heading in radians, clockwise from right.
    pub heading: f32,
    /// Forward speed, world units per second.
    pub speed: f32,
    /// Elapsed simulation seconds.
    pub elapsed: f32,
    /// Completed laps and checkpoint state.
    pub progress: Progress,
    velocity: [f32; 2],
}

impl Default for Race {
    fn default() -> Self {
        Self {
            position: [0.0, -RADIUS],
            heading: 0.0,
            speed: 0.0,
            elapsed: 0.0,
            progress: Progress::default(),
            velocity: [0.0; 2],
        }
    }
}

impl Race {
    /// Advance a validated step of at most 1/20 second.
    ///
    /// # Errors
    /// Refuses non-finite, negative or oversized time, invalid controls/settings.
    pub fn step(&mut self, dt: f32, controls: Controls, settings: Settings) -> Result<(), String> {
        settings::validate(dt, controls, settings)?;
        if self.progress.laps == 3 {
            return Ok(());
        }
        self.elapsed += dt;
        self.drive(dt, controls, settings);
        self.progress.observe(self.position, self.elapsed);
        Ok(())
    }

    fn drive(&mut self, dt: f32, input: Controls, settings: Settings) {
        let grass = road_distance(self.position) > ROAD;
        let drag = if grass { 2.6 } else { 0.33 };
        let acceleration = input.throttle * 115.0 - input.brake * 220.0;
        self.speed = ((self.speed + acceleration * dt) / (1.0 + drag * dt)).clamp(0.0, MAX_SPEED);
        let turn = (self.speed / 70.0).min(1.0) * 1.65 * settings.steering;
        self.heading = (self.heading + input.steer * turn * dt).rem_euclid(std::f32::consts::TAU);
        let grip = (settings.grip * if grass { 3.0 } else { 8.0 } * dt).min(1.0);
        for (axis, direction) in [self.heading.cos(), self.heading.sin()]
            .into_iter()
            .enumerate()
        {
            self.velocity[axis] += (direction * self.speed - self.velocity[axis]) * grip;
            self.position[axis] += self.velocity[axis] * dt;
        }
    }
}
