//! Platform app: routes input, advances simulation, paints world and compiled HUD.
use crate::{
    driving::Driving,
    gpu::Track,
    hud::{Hud, HudProps},
    model::{Camera, Race},
};
use std::{cell::RefCell, rc::Rc};
use vapors_app::{DocumentInput, ScenePainter};
use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Instant, Next, Platform, report_error};
use vapors_text::FontSystem;

pub(crate) struct Racer {
    pub(super) document: Rc<RefCell<Document>>,
    pub(super) hud: Hud,
    pub(super) input: DocumentInput,
    pub(super) driving: Driving,
    pub(super) race: Race,
    pub(super) camera: Camera,
    pub(super) paused: bool,
    pub(super) last: Option<Instant>,
    pub(super) remainder: f32,
    pub(super) fps_elapsed: f32,
    pub(super) fps_frames: f32,
    pub(super) scale: f32,
    pub(super) pointer: [f32; 2],
    pub(super) inspect: bool,
    pub(super) error: Option<String>,
    pub(super) setup_error: Option<String>,
    pub(super) track: Option<Track>,
    pub(super) painter: Option<ScenePainter>,
}
impl Racer {
    pub(crate) fn new() -> Self {
        let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
        let body = document.borrow().body();
        let hud = Hud::mount(&document, body, HudProps {});
        Self {
            document,
            hud,
            input: DocumentInput::new(),
            driving: Driving::default(),
            race: Race::default(),
            camera: Camera::default(),
            paused: true,
            last: None,
            remainder: 0.0,
            fps_elapsed: 0.0,
            fps_frames: 0.0,
            scale: 1.0,
            pointer: [0.0; 2],
            inspect: vapors_platform::query_parameter("inspect").as_deref() == Some("1"),
            error: None,
            setup_error: None,
            track: None,
            painter: None,
        }
    }
    pub(super) fn fail(&mut self, error: String) {
        report_error(&error);
        self.error = Some(error);
        self.paused = true;
        self.driving.clear();
    }
    pub(super) fn pause(&mut self) {
        self.paused = true;
        self.camera.moving = false;
        self.driving.clear();
        self.last = None;
        self.remainder = 0.0;
        self.fps_elapsed = 0.0;
        self.fps_frames = 0.0;
        self.hud.fps.set("—".into());
    }
    pub(super) fn command(&mut self, command: &str) -> Result<(), String> {
        match command {
            "" => {}
            "toggle" => {
                if self.race.progress.laps == 3 {
                    self.race = Race::default();
                    self.camera = Camera::default();
                }
                if self.paused {
                    self.resume();
                } else {
                    self.pause();
                }
            }
            "camera" => {
                self.camera.toggle();
                self.last = None;
                self.hud.follow.set(self.camera.follow);
            }
            "restart" => {
                self.race = Race::default();
                self.camera = Camera::default();
                self.pause();
            }
            "tune" => {
                self.pause();
                self.hud.settings.set(!self.hud.settings.get());
            }
            _ => {
                return Err(format!(
                    "unknown racer command `{command}`; use toggle, restart, tune or camera"
                ));
            }
        }
        Ok(())
    }
}
impl App for Racer {
    fn event(&mut self, event: Event, platform: &Platform) {
        if self.error.is_some() {
            return;
        }
        if let Err(error) = self.handle(&event, platform) {
            self.fail(error);
        }
        platform.request_redraw();
    }
    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        if self.error.is_some() {
            return Next::Wait;
        }
        if let Err(error) = self.draw(frame) {
            self.fail(error);
            return Next::Wait;
        }
        if self.paused && !self.camera.moving {
            Next::Wait
        } else {
            Next::Continue
        }
    }
    fn drawn(&mut self, platform: &Platform) {
        crate::text_input::mirror(&self.document, platform);
    }
}
mod drawing;
mod input;

mod inspection;

mod tuning;
