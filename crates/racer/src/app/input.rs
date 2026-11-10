//! Document events update component signals; the app handles driving and commands.
use super::Racer;
use vapors_app::{Route, Unplaced};
use vapors_platform::{Event, Platform};
impl Racer {
    pub(crate) fn handle(&mut self, event: &Event, platform: &Platform) -> Result<(), String> {
        if matches!(event, Event::WindowFocus { focused: false }) {
            self.pause();
        }
        let Some(route) = self.give(event, platform)? else {
            return Ok(());
        };
        crate::text_input::edit(&self.document, event)?;
        self.routed_key(event, route)?;
        self.run_hud_action()?;
        self.sync_driving_controls();
        crate::text_input::mirror(&self.document, platform);
        Ok(())
    }
    /// The document's turn at `event`: where it goes next, or `None`
    /// when it was dropped before the first frame.
    fn give(&mut self, event: &Event, platform: &Platform) -> Result<Option<Route>, String> {
        match self.input.give(&self.document, event, platform) {
            Ok(given) => {
                if let Some(cursor) = given.cursor {
                    platform.set_cursor(cursor);
                }
                Ok(Some(given.route))
            }
            // RELAXED: before the first frame the document has no layout,
            // so an event has no target and is dropped, a key included.
            // Only that window is let through: after the first frame
            // NotUpdated is an error. DocumentInput holding events until
            // the first update would retire this.
            Err(Unplaced::NotUpdated) if self.gpu.track.is_none() => Ok(None),
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
            if !self.run.paused {
                self.run.driving.key(code, *pressed);
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
        if self.run.paused
            && held.iter().any(|(_, down)| *down)
            && !self.hud.settings.get()
            && self.run.race.progress.laps < 3
        {
            self.resume();
        }
        for (name, down) in held {
            self.run.driving.hold(
                format!("ui-{name}"),
                (down && !self.run.paused).then(|| name.to_owned()),
            );
        }
    }
}
