//! Input and tuning validation at the simulation boundary.
/// Normalised driving input.
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    /// Acceleration, 0..=1.
    pub throttle: f32,
    /// Braking, 0..=1.
    pub brake: f32,
    /// Left -1, centred 0, right +1.
    pub steer: f32,
}
/// User-adjustable steering and lateral grip.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Steering multiplier, 0.5..=1.5.
    pub steering: f32,
    /// Grip multiplier, 0.5..=1.5.
    pub grip: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            steering: 1.0,
            grip: 1.0,
        }
    }
}
pub(super) fn validate(dt: f32, input: Controls, settings: Settings) -> Result<(), String> {
    for (name, value, low, high) in [
        ("timestep", dt, 0.0, 0.05),
        ("throttle", input.throttle, 0.0, 1.0),
        ("brake", input.brake, 0.0, 1.0),
        ("steering input", input.steer, -1.0, 1.0),
        ("steering setting", settings.steering, 0.5, 1.5),
        ("grip setting", settings.grip, 0.5, 1.5),
    ] {
        if !value.is_finite() || !(low..=high).contains(&value) {
            return Err(format!(
                "racer {name} {value} refused; provide a finite value in {low}..={high}"
            ));
        }
    }
    Ok(())
}

impl Settings {
    /// Validate tuning without advancing the simulation.
    ///
    /// # Errors
    /// Non-finite or out-of-range steering/grip values.
    pub fn validate(self) -> Result<(), String> {
        validate(0.0, Controls::default(), self)
    }
}
