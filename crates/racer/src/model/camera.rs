//! Smooth fixed-orientation and heading-following camera poses.
use super::{Controls, MAX_SPEED, Settings};
/// Camera scale: smaller means more world visible.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Current scale, including speed and camera mode.
    pub zoom: f32,
    /// Whether the camera follows the car's heading.
    pub follow: bool,
    /// Camera yaw in radians.
    pub rotation: f32,
    /// Cosine of tilt away from vertical.
    pub tilt: f32,
    /// World-space offset ahead of the car.
    pub offset: [f32; 2],
    /// True until the current target is reached.
    pub moving: bool,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            follow: false,
            rotation: 0.0,
            tilt: 0.82,
            offset: [0.0; 2],
            moving: false,
        }
    }
}
impl Camera {
    /// Switch mode without changing the current view immediately.
    pub fn toggle(&mut self) {
        self.follow = !self.follow;
        self.moving = true;
    }

    /// Ease toward the speed's view scale.
    ///
    /// # Errors
    /// Invalid timestep, speed or heading is refused before changing the camera.
    pub fn step(&mut self, dt: f32, speed: f32, heading: f32) -> Result<(), String> {
        super::settings::validate(dt, Controls::default(), Settings::default())?;
        if !speed.is_finite() || !(0.0..=MAX_SPEED).contains(&speed) {
            return Err(format!(
                "racer camera speed {speed} refused; expected 0..={MAX_SPEED}"
            ));
        }
        if !heading.is_finite() {
            return Err(format!(
                "racer camera heading {heading} refused; expected a finite angle"
            ));
        }
        self.blend(dt, speed, heading);
        Ok(())
    }
    fn blend(&mut self, dt: f32, speed: f32, heading: f32) {
        // Three inverse seconds matches the existing speed-zoom response.
        let amount = 1.0 - (-3.0 * dt).exp();
        let zoom = (1.0 - 0.32 * (speed / MAX_SPEED)) * if self.follow { 1.25 } else { 1.0 };
        let tilt = if self.follow { 0.62 } else { 0.82 };
        let yaw = if self.follow {
            heading + std::f32::consts::FRAC_PI_2
        } else {
            0.0
        };
        let look = speed * 0.20 + if self.follow { 45.0 } else { 0.0 };
        let offset = [heading.cos() * look, heading.sin() * look];
        self.zoom = approach(self.zoom, zoom, amount);
        self.tilt = approach(self.tilt, tilt, amount);
        self.rotation = wrap(approach(
            self.rotation,
            self.rotation + wrap(yaw - self.rotation),
            amount,
        ));
        for (current, target) in self.offset.iter_mut().zip(offset) {
            *current = approach(*current, target, amount);
        }
        self.moving = (self.zoom - zoom).abs() > 0.001
            || (self.tilt - tilt).abs() > 0.001
            || wrap(yaw - self.rotation).abs() > 0.001
            || self
                .offset
                .iter()
                .zip(offset)
                .any(|(a, b)| (*a - b).abs() > 0.001);
    }
}
fn approach(current: f32, target: f32, amount: f32) -> f32 {
    let next = current + (target - current) * amount;
    if (target - next).abs() < 0.001 {
        target
    } else {
        next
    }
}
fn wrap(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}
