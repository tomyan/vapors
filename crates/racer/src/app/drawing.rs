//! Fixed-step simulation and GPU world with compiled document overlay.
use super::Racer;
use crate::gpu::Track;
use vapors_app::ScenePainter;
use vapors_elements::{Document, Viewport};
use vapors_platform::Frame;
use vapors_renderer::Target;
use vapors_scene::Size;
impl Racer {
    pub(super) fn advance(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        let elapsed = self
            .last
            .replace(frame.now)
            .map_or(0.0, |last| frame.now.duration_since(last).as_secs_f32());
        if self.paused {
            return Ok(());
        }
        self.advance_elapsed(elapsed)
    }
    pub(crate) fn advance_elapsed(&mut self, elapsed: f32) -> Result<(), String> {
        if !elapsed.is_finite() || elapsed < 0.0 {
            return Err("racer frame duration must be finite and nonnegative".into());
        }
        self.sample_fps(elapsed);
        // Cap catch-up at six 120Hz steps; slow rendering must not release
        // held pedals. Actual focus loss pauses through the input handler.
        self.remainder += elapsed.min(0.05);
        let settings = self.settings();
        while self.remainder >= 1.0 / 120.0 {
            self.race
                .step(1.0 / 120.0, self.driving.controls(), settings)?;
            self.camera.step(1.0 / 120.0, self.race.speed)?;
            self.remainder -= 1.0 / 120.0;
        }
        if self.race.progress.laps == 3 {
            self.pause();
        }
        Ok(())
    }
    fn sample_fps(&mut self, elapsed: f32) {
        if elapsed == 0.0 {
            return;
        }
        self.fps_elapsed += elapsed;
        self.fps_frames += 1.0;
        // Half-second display averaging keeps the readout legible while driving.
        if self.fps_elapsed >= 0.5 {
            self.hud
                .fps
                .set(format!("{:.0}", self.fps_frames / self.fps_elapsed));
            self.fps_elapsed = 0.0;
            self.fps_frames = 0.0;
        }
    }
    pub(super) fn draw(&mut self, frame: &Frame<'_>) -> Result<(), String> {
        self.advance(frame)?;
        #[allow(clippy::cast_possible_truncation)] // display scale is a small positive number
        {
            self.scale = frame.scale_factor as f32;
        }
        let size = frame.texture.size();
        #[allow(clippy::cast_precision_loss)] // viewport dimensions fit f32
        let viewport = Viewport {
            width: size.width as f32 / self.scale,
            height: size.height as f32 / self.scale,
            scale: self.scale,
        };
        self.sync_hud(viewport.width);
        Document::settle_selection(&self.document);
        self.document
            .borrow_mut()
            .update(viewport)
            .map_err(|errors| format!("racer HUD refused: {errors:?}"))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if self.track.is_none() {
            self.track = Some(Track::new(frame));
        }
        if self.painter.is_none() {
            self.painter = Some(
                ScenePainter::new(frame.device, frame.texture.format())
                    .map_err(|e| e.to_string())?,
            );
        }
        self.track.as_ref().expect("created").draw(
            frame,
            &view,
            &self.race,
            self.camera,
            self.driving.controls().brake,
        );
        let target = Target {
            device: frame.device,
            queue: frame.queue,
            view: &view,
            viewport: Size {
                width: size.width,
                height: size.height,
            },
            scissor: None,
        };
        let document = self.document.borrow();
        self.painter
            .as_mut()
            .expect("created")
            .draw_over(&target, document.painted().expect("updated"))
            .map_err(|e| e.to_string())?;
        self.report_frame(frame.index);
        Ok(())
    }
    fn sync_hud(&self, width: f32) {
        self.hud.compact.set(width < 760.0);
        self.hud.paused.set(self.paused);
        self.hud.speed.set(format!("{:.0}", self.race.speed * 0.72));
        self.hud
            .lap
            .set(format!("{} / 3", (self.race.progress.laps + 1).min(3)));
        self.hud.clock.set(time(self.race.elapsed));
        self.hud
            .best
            .set(self.race.progress.best.map_or_else(|| "—".into(), time));
        let message = if self.setup_error.is_some() {
            "Steering and grip must be between 0.5 and 1.5."
        } else if self.race.progress.laps == 3 {
            "Finished. Restart for another run."
        } else if self.hud.settings.get() {
            "Paused · adjust the car, then Drive."
        } else if self.paused {
            "Drive to start · clockwise · three clean laps"
        } else if crate::model::road_distance(self.race.position) > crate::model::ROAD + 6.0 {
            "Off track · return for a clean lap"
        } else {
            "Clockwise · camera widens with speed"
        };
        self.hud.message.set(message.into());
    }
}
fn time(seconds: f32) -> String {
    let minutes = (seconds / 60.0).floor();
    format!("{minutes:02.0}:{:05.2}", seconds % 60.0)
}
