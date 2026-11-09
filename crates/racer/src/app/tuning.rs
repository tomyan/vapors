//! Refuse invalid car setup without ending the editing session.
use super::Racer;
use crate::model::Settings;
impl Racer {
    #[allow(clippy::cast_possible_truncation)] // values are validated before use
    pub(super) fn settings(&self) -> Settings {
        Settings {
            steering: self.hud.steering.get() as f32,
            grip: self.hud.grip.get() as f32,
        }
    }
    pub(super) fn resume(&mut self) {
        if let Err(error) = self.settings().validate() {
            self.setup_error = Some(error);
            self.hud.settings.set(true);
            return;
        }
        self.setup_error = None;
        self.paused = false;
        self.last = None;
        self.hud.settings.set(false);
    }
}
