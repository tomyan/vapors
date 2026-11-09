//! Document events update component signals; the app handles driving and commands.
use super::Racer;
use vapors_app::{Route, Unplaced};
use vapors_platform::{Event, Platform};
impl Racer {
    pub(super) fn handle(&mut self, event: &Event, platform: &Platform) -> Result<(), String> {
        if matches!(event, Event::WindowFocus { focused: false }) {
            self.pause();
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
        self.sync_driving_controls();
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
    pub(crate) fn sync_driving_controls(&mut self) {
        let held = [
            ("left", self.hud.left.get()),
            ("right", self.hud.right.get()),
            ("brake", self.hud.brake.get()),
            ("gas", self.hud.gas.get()),
        ];
        if self.paused
            && held.iter().any(|(_, down)| *down)
            && !self.hud.settings.get()
            && self.race.progress.laps < 3
        {
            self.resume();
        }
        for (name, down) in held {
            self.driving.hold(
                format!("ui-{name}"),
                (down && !self.paused).then(|| name.to_owned()),
            );
        }
    }
}
