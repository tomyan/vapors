//! Platform app: routes input, advances simulation, and shows the compiled HUD
//! through an `ElementsApp`, the track drawn in its `gpu-surface`.
use crate::{
    hud::{Hud, HudProps},
    model::{Camera, Race},
};
use state::{Faults, Pacing, Run};
use std::{cell::RefCell, rc::Rc};
use surface::TrackSurface;
use vapors_app::{ElementsApp, Failure};
use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Next, Platform, report_error};
use vapors_text::FontSystem;

pub(crate) struct Racer {
    pub(super) document: Rc<RefCell<Document>>,
    pub(super) hud: Hud,
    pub(super) app: ElementsApp,
    pub(super) track: TrackSurface,
    pub(super) run: Run,
    pub(super) pacing: Pacing,
    pub(super) faults: Faults,
    /// A frame has been drawn: the document has been laid out.
    pub(super) framed: bool,
    pub(super) inspect: bool,
}
impl Racer {
    pub(crate) fn new() -> Self {
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
            inspect: vapors_platform::query_parameter("inspect").as_deref() == Some("1"),
        }
    }
    /// Where the HUD's `ElementsApp` keeps why it stopped: `run` reads
    /// it after the loop.
    pub(crate) fn failure(&self) -> Failure {
        self.app.failure()
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
