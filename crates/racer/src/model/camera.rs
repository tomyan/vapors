//! Speed-dependent view scale, smoothed independently of refresh rate.
use super::{Controls, MAX_SPEED, Settings};
/// Camera scale: smaller means more world visible.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    /// Current scale, 0.68..=1.
    pub zoom: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self { zoom: 1.0 }
    }
}
impl Camera {
    /// Ease toward the speed's view scale.
    ///
    /// # Errors
    /// Invalid timestep or speed is refused before changing the camera.
    pub fn step(&mut self, dt: f32, speed: f32) -> Result<(), String> {
        super::settings::validate(dt, Controls::default(), Settings::default())?;
        if !speed.is_finite() || !(0.0..=MAX_SPEED).contains(&speed) {
            return Err(format!(
                "racer camera speed {speed} refused; expected 0..={MAX_SPEED}"
            ));
        }
        let target = 1.0 - 0.32 * (speed / MAX_SPEED);
        self.zoom += (target - self.zoom) * (1.0 - (-3.0 * dt).exp());
        Ok(())
    }
}
