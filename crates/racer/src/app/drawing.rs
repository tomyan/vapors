//! Fixed-step simulation, and what the frame shows: the HUD's values and the track's scene.
use super::Racer;
use crate::gpu::Scene;
use vapors_elements::Viewport;
use vapors_platform::Frame;
impl Racer {
    pub(super) fn advance(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        let elapsed = self.frame_elapsed(frame);
        if !self.run.paused {
            self.advance_elapsed(elapsed)?;
        }
        if !self.run.paused || self.run.camera.moving {
            self.step_camera(elapsed)?;
        }
        Ok(())
    }
    fn frame_elapsed(&mut self, frame: &Frame<'_>) -> f32 {
        let last = self.pacing.last.replace(frame.now);
        last.map_or(0.0, |last| frame.now.duration_since(last).as_secs_f32())
    }
    fn step_camera(&mut self, elapsed: f32) -> Result<(), String> {
        let (speed, heading) = (self.run.race.speed, self.run.race.heading);
        self.run.camera.step(elapsed.min(0.05), speed, heading)
    }
    pub(crate) fn advance_elapsed(&mut self, elapsed: f32) -> Result<(), String> {
        check_elapsed(elapsed)?;
        self.sample_fps(elapsed);
        // Cap catch-up at six 120Hz steps; slow rendering must not release
        // held pedals. Actual focus loss pauses through the input handler.
        self.pacing.remainder += elapsed.min(0.05);
        self.step_fixed()?;
        if self.run.race.progress.laps == 3 {
            self.pause();
        }
        Ok(())
    }
    fn step_fixed(&mut self) -> Result<(), String> {
        let settings = self.settings();
        while self.pacing.remainder >= 1.0 / 120.0 {
            self.run
                .race
                .step(1.0 / 120.0, self.run.driving.controls(), settings)?;
            self.pacing.remainder -= 1.0 / 120.0;
        }
        Ok(())
    }
    fn sample_fps(&mut self, elapsed: f32) {
        if elapsed == 0.0 {
            return;
        }
        self.pacing.fps_elapsed += elapsed;
        self.pacing.fps_frames += 1.0;
        // Half-second display averaging keeps the readout legible while driving.
        if self.pacing.fps_elapsed >= 0.5 {
            self.hud.fps.set_if_changed(format!(
                "{:.0}",
                self.pacing.fps_frames / self.pacing.fps_elapsed
            ));
            self.pacing.fps_elapsed = 0.0;
            self.pacing.fps_frames = 0.0;
        }
    }
    /// Advances the race and writes what the frame shows, before the
    /// `ElementsApp` updates and paints the document.
    pub(super) fn prepare(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        self.advance(frame)?;
        let viewport = self.viewport(frame);
        self.sync_hud(viewport);
        let moving = !self.run.paused || self.run.camera.moving;
        self.track
            .show(self.scene(), moving)
            .map_err(|error| format!("the racer's track surface refused: {error}"))
    }
    pub(crate) fn scene(&self) -> Scene {
        Scene {
            race: self.run.race.clone(),
            camera: self.run.camera,
            brake: self.run.driving.controls().brake,
        }
    }
    fn viewport(&mut self, frame: &Frame<'_>) -> Viewport {
        #[allow(clippy::cast_possible_truncation)] // display scale is a small positive number
        {
            self.pacing.scale = frame.scale_factor as f32;
        }
        let size = frame.texture.size();
        #[allow(clippy::cast_precision_loss)] // viewport dimensions fit f32
        Viewport {
            width: size.width as f32 / self.pacing.scale,
            height: size.height as f32 / self.pacing.scale,
            scale: self.pacing.scale,
        }
    }
    pub(super) fn sync_hud(&self, viewport: Viewport) {
        self.hud.short.set_if_changed(viewport.height < 480.0);
        self.hud.compact.set_if_changed(viewport.width < 760.0);
        self.hud.follow.set_if_changed(self.run.camera.follow);
        let enabled = !self.hud.settings.get() && self.run.race.progress.laps < 3;
        self.hud.enabled.set_if_changed(enabled);
        self.hud.paused.set_if_changed(self.run.paused);
        self.sync_readouts();
        self.hud.message.set_if_changed(self.message().into());
    }
    fn sync_readouts(&self) {
        self.hud
            .speed
            .set_if_changed(format!("{:.0}", self.run.race.speed * 0.72));
        self.hud
            .lap
            .set_if_changed(format!("{} / 3", (self.run.race.progress.laps + 1).min(3)));
        self.hud.clock.set_if_changed(time(self.run.race.elapsed));
        self.hud
            .best
            .set_if_changed(self.run.race.progress.best.map_or_else(|| "—".into(), time));
    }
    fn message(&self) -> &'static str {
        if self.faults.setup_error.is_some() {
            "Steering and grip must be between 0.5 and 1.5."
        } else if self.run.race.progress.laps == 3 {
            "Finished. Restart for another run."
        } else if self.hud.settings.get() {
            "Paused · adjust the car, then Drive."
        } else if self.run.paused {
            "Drive to start · clockwise · three clean laps"
        } else if crate::model::road_distance(self.run.race.position) > crate::model::ROAD + 6.0 {
            "Off track · return for a clean lap"
        } else {
            "Clockwise · camera widens with speed"
        }
    }
}
fn check_elapsed(elapsed: f32) -> Result<(), String> {
    if elapsed.is_finite() && elapsed >= 0.0 {
        return Ok(());
    }
    Err("racer frame duration must be finite and nonnegative".into())
}
fn time(seconds: f32) -> String {
    let minutes = (seconds / 60.0).floor();
    format!("{minutes:02.0}:{:05.2}", seconds % 60.0)
}
