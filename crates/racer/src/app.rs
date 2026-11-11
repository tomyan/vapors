//! Platform app: routes input, advances simulation, and shows the compiled HUD
//! through an `ElementsApp`, the track drawn in its `gpu-surface`.
use crate::{
    hud::{Hud, HudProps},
    model::{Camera, Race},
};
use state::{Faults, Pacing, Run};
use std::{cell::RefCell, rc::Rc};
use surface::TrackSurface;
use vapors_app::{ElementsApp, Failure, FrameCounts, FrameTiming};
use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Next, Platform, report_error};
use vapors_text::FontSystem;

/// The racer as a platform app: the HUD's document and the track's
/// surface, paused until Space. Public for drivers outside a window
/// (the timing harness in `crates/perf`).
pub struct Racer {
    pub(super) document: Rc<RefCell<Document>>,
    pub(super) hud: Hud,
    pub(super) app: ElementsApp,
    pub(super) track: TrackSurface,
    pub(super) run: Run,
    pub(super) pacing: Pacing,
    pub(super) faults: Faults,
    /// A frame has been drawn: the document has been laid out.
    pub(super) framed: bool,
    /// The last `prepare`'s HUD writes noted a document change, which
    /// it takes so they ask for no next frame.
    // RELAXED: read only by the IU5 tests (gpu_hud_iu5_tests.rs) until
    // the inspector reports it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) hud_changed: bool,
    pub(super) inspect: bool,
}
impl Default for Racer {
    fn default() -> Self {
        Self::new()
    }
}
impl Racer {
    /// A paused racer whose HUD is mounted and not yet drawn.
    #[must_use]
    pub fn new() -> Self {
        let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
        let body = document.borrow().body();
        let hud = Hud::mount(&document, body, HudProps {});
        let (app, track) = TrackSurface::mount(ElementsApp::new(Rc::clone(&document)));
        Self {
            document,
            hud,
            app,
            track,
            run: Run::default(),
            pacing: Pacing::default(),
            faults: Faults::default(),
            framed: false,
            hud_changed: false,
            inspect: vapors_platform::query_parameter("inspect").as_deref() == Some("1"),
        }
    }
    /// Where the HUD's `ElementsApp` keeps why it stopped: `run` reads
    /// it after the loop.
    pub(crate) fn failure(&self) -> Failure {
        self.app.failure()
    }
    /// The HUD's document, for its update counts.
    #[must_use]
    pub fn document(&self) -> Rc<RefCell<Document>> {
        Rc::clone(&self.document)
    }
    /// What the HUD's last frame handed the GPU.
    #[must_use]
    pub fn frame_counts(&self) -> FrameCounts {
        self.app.frame_counts()
    }
    /// The HUD's frame times, its track's render included.
    #[must_use]
    pub fn frame_timing(&self) -> FrameTiming {
        self.app.frame_timing()
    }
    /// Render passes the HUD's last frame began.
    #[must_use]
    pub fn render_passes(&self) -> u64 {
        self.app.render_passes()
    }
    /// Why the racer stopped, its own fault or the HUD's; `None` while it runs.
    #[must_use]
    pub fn error(&self) -> Option<String> {
        self.faults.error.clone().or_else(|| self.failure().get())
    }
    fn hud_failure(&self) -> String {
        self.failure().get().unwrap_or_else(|| {
            "the HUD's ElementsApp ended the loop without a failure; it exits only on one".into()
        })
    }
    pub(super) fn fail(&mut self, error: String) {
        report_error(&error);
        self.faults.error = Some(error);
        self.run.paused = true;
        self.run.driving.clear();
        self.hud.reset.update(|epoch| *epoch += 1);
    }
    pub(super) fn pause(&mut self) {
        self.run.paused = true;
        self.run.camera.moving = false;
        self.run.driving.clear();
        self.hud.reset.update(|epoch| *epoch += 1);
        self.pacing.last = None;
        self.pacing.remainder = 0.0;
        self.pacing.fps_elapsed = 0.0;
        self.pacing.fps_frames = 0.0;
        self.hud.fps.set("—".into());
    }
    pub(super) fn command(&mut self, command: &str) -> Result<(), String> {
        match command {
            "" => {}
            "toggle" => self.toggle(),
            "camera" => self.toggle_camera(),
            "restart" => self.restart(),
            "tune" => self.tune(),
            _ => return Err(unknown_command(command)),
        }
        Ok(())
    }
    fn toggle(&mut self) {
        if self.run.race.progress.laps == 3 {
            self.run.race = Race::default();
            self.run.camera = Camera::default();
        }
        if self.run.paused {
            self.resume();
        } else {
            self.pause();
        }
    }
    fn toggle_camera(&mut self) {
        self.run.camera.toggle();
        self.pacing.last = None;
        self.hud.follow.set(self.run.camera.follow);
    }
    fn restart(&mut self) {
        self.run.race = Race::default();
        self.run.camera = Camera::default();
        self.pause();
    }
    fn tune(&mut self) {
        self.pause();
        self.hud.settings.set(!self.hud.settings.get());
    }
}
impl App for Racer {
    fn event(&mut self, event: Event, platform: &Platform) {
        if self.faults.error.is_some() {
            return;
        }
        if let Err(error) = self.handle(&event, platform) {
            self.fail(error);
        }
        platform.request_redraw();
    }
    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        if self.faults.error.is_some() {
            return Next::Wait;
        }
        if let Err(error) = self.prepare(frame) {
            self.fail(error);
            return Next::Wait;
        }
        let next = self.app.frame(frame);
        if next == Next::Exit {
            // the ElementsApp failed and closed its surfaces: the racer ends
            self.fail(self.hud_failure());
            return Next::Exit;
        }
        self.framed = true;
        if let Err(error) = self.after_frame(frame.index) {
            self.fail(error);
            return Next::Wait;
        }
        next
    }
    fn drawn(&mut self, platform: &Platform) {
        self.app.drawn(platform);
    }
}
mod drawing;
#[cfg(test)]
mod hud_frame_tests;
mod input;
#[cfg(test)]
mod input_tests;
mod state;
mod surface;

mod inspection;
#[cfg(test)]
mod inspection_tests;

mod tuning;

fn unknown_command(command: &str) -> String {
    format!("unknown racer command `{command}`; use toggle, restart, tune or camera")
}
