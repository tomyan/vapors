//! Document events update component signals; the app handles driving and commands.
use super::Racer;
use vapors_app::{GiveError, Route, Unplaced};
use vapors_platform::{Event, Platform};
impl Racer {
    pub(crate) fn handle(&mut self, event: &Event, platform: &Platform) -> Result<(), String> {
        if matches!(event, Event::WindowFocus { focused: false }) {
            self.pause();
        }
        let Some(route) = self.give(event, platform)? else {
            return Ok(());
        };
        self.routed_key(event, route)?;
        self.run_hud_action()?;
        self.sync_driving_controls();
        Ok(())
    }
    /// The document's turn at `event`: where it goes next, or `None`
    /// when it was dropped before the first frame.
    fn give(&mut self, event: &Event, platform: &Platform) -> Result<Option<Route>, String> {
        match self.app.give(event, platform) {
            Ok(route) => Ok(Some(route)),
            // RELAXED: before the first frame the document has no layout,
            // so an event has no target and is dropped, a key included.
            // Only that window is let through: after the first frame
            // NotUpdated is an error. ElementsApp holding events until
            // the first update would retire this.
            Err(GiveError::Unplaced(Unplaced::NotUpdated)) if !self.framed => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
    fn run_hud_action(&mut self) -> Result<(), String> {
        let command = self.hud.action.get();
        if command.is_empty() {
            return Ok(());
        }
        self.hud.action.set(String::new());
        self.command(&command)
    }
    /// The app's keys, unless the document used the key. A click on a
    /// `HoldButton` leaves focus where it was (its pointerdown is
    /// prevented), so Space still reaches the app afterwards.
    pub(crate) fn routed_key(&mut self, event: &Event, route: Route) -> Result<(), String> {
        if route != Route::Document {
            self.key(event)?;
        }
        Ok(())
    }
    fn key(&mut self, event: &Event) -> Result<(), String> {
        let Some((code, pressed, repeat)) = key_parts(event) else {
            return Ok(());
        };
        if pressed && !repeat {
            self.key_command(code)?;
        }
        if !self.run.paused {
            self.run.driving.key(code, pressed);
        }
        Ok(())
    }
    fn key_command(&mut self, code: &str) -> Result<(), String> {
        match code {
            "Space" | "Escape" => self.command("toggle"),
            "KeyR" => self.command("restart"),
            _ => Ok(()),
        }
    }
    pub(crate) fn sync_driving_controls(&mut self) {
        let held = self.held_buttons();
        if self.run.paused && held.iter().any(|(_, down)| *down) && self.can_resume() {
            self.resume();
        }
        for (name, down) in held {
            let active = (down && !self.run.paused).then(|| name.to_owned());
            self.run.driving.hold(format!("ui-{name}"), active);
        }
    }
    fn held_buttons(&self) -> [(&'static str, bool); 4] {
        [
            ("left", self.hud.left.get()),
            ("right", self.hud.right.get()),
            ("brake", self.hud.brake.get()),
            ("gas", self.hud.gas.get()),
        ]
    }
    fn can_resume(&self) -> bool {
        !self.hud.settings.get() && self.run.race.progress.laps < 3
    }
}
fn key_parts(event: &Event) -> Option<(&str, bool, bool)> {
    match event {
        Event::Key {
            code,
            pressed,
            repeat,
            ..
        } => Some((code, *pressed, *repeat)),
        _ => None,
    }
}
