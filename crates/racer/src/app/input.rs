//! Driving controls use hit-tested Vapors buttons; all other input goes to the document.
use super::Racer;
use vapors_app::{Route, Unplaced};
use vapors_platform::{Event, Platform, TouchPhase};
impl Racer {
    pub(super) fn handle(&mut self, event: &Event, platform: &Platform) -> Result<(), String> {
        if matches!(event, Event::WindowFocus { focused: false }) {
            self.pause();
        }
        if self.drive_pointer(event) {
            return Ok(());
        }
        let route = match self.input.give(&self.document, event, platform) {
            Ok(given) => {
                if let Some(cursor) = given.cursor {
                    platform.set_cursor(cursor);
                }
                given.route
            }
            Err(Unplaced::NotUpdated) if self.track.is_none() => return Ok(()),
            Err(error) => return Err(error.to_string()),
        };
        crate::text_input::edit(&self.document, event)?;
        if route != Route::Document {
            self.key(event)?;
        }
        let command = self.hud.action.get();
        if !command.is_empty() {
            self.hud.action.set(String::new());
            self.command(&command)?;
        }
        crate::text_input::mirror(&self.document, platform);
        Ok(())
    }
    fn key(&mut self, event: &Event) -> Result<(), String> {
        if let Event::Key {
            code,
            pressed,
            repeat,
            ..
        } = event
        {
            if *pressed && !repeat {
                match code.as_str() {
                    "Space" | "Escape" => self.command("toggle")?,
                    "KeyR" => self.command("restart")?,
                    _ => {}
                }
            }
            if !self.paused {
                self.driving.key(code, *pressed);
            }
        }
        Ok(())
    }
    fn drive_pointer(&mut self, event: &Event) -> bool {
        match *event {
            Event::PointerMoved { x, y } => {
                self.pointer = [x / self.scale, y / self.scale];
                false
            }
            Event::PrimaryButton { pressed, .. } => {
                self.hold_pointer("mouse".into(), self.pointer, pressed)
            }
            Event::PointerLeft => {
                self.driving.hold("mouse".into(), None);
                false
            }
            Event::Touch { id, phase, x, y } => {
                let name = format!("touch-{id}");
                match phase {
                    TouchPhase::Started => {
                        self.hold_pointer(name, [x / self.scale, y / self.scale], true)
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.hold_pointer(name, [x / self.scale, y / self.scale], false)
                    }
                    TouchPhase::Moved => self.driving.contains(&name),
                }
            }
            _ => false,
        }
    }
    fn hold_pointer(&mut self, id: String, point: [f32; 2], pressed: bool) -> bool {
        if !pressed {
            let held = self.driving.contains(&id);
            self.driving.hold(id, None);
            return held;
        }
        let control = {
            let doc = self.document.borrow();
            doc.hit(point)
                .and_then(|node| doc.attribute(node, "data-drive"))
                .map(str::to_owned)
        };
        if control.is_none() {
            return false;
        }
        if self.paused && !self.hud.settings.get() && self.race.progress.laps < 3 {
            self.resume();
        }
        if !self.paused {
            self.driving.hold(id, control);
        }
        true
    }
}
