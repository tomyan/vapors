//! The racer driven by the script (`driver.rs`) through a player's
//! input: the controls as held keys, Space to start again after three
//! laps, and a click on the HUD's camera button. Each scripted change
//! is checked to have happened, and every frame that the racer races
//! under the keys sent (`racer_driven_workload.rs`); one that did not
//! ends the run.

use std::collections::BTreeSet;

use vapors_racer::Racer;
use vapors_racer::model::Controls;

use crate::driver::{Scripted, controls, restart_due, scripted};
use crate::racer_input::{centre_of, click, focused, key};

/// The racer, racing from its first frame, driven by the script.
pub struct DrivenRacer {
    racer: Racer,
    held: BTreeSet<&'static str>,
    fault: Option<String>,
}

impl Default for DrivenRacer {
    fn default() -> Self {
        Self::new()
    }
}

impl DrivenRacer {
    /// A racer timing its frames on the GPU, started with Space.
    #[must_use]
    pub fn new() -> Self {
        let mut racer = Racer::new();
        racer.time_frames(true);
        key(&mut racer, "Space", true);
        key(&mut racer, "Space", false);
        Self {
            racer,
            held: BTreeSet::new(),
            fault: None,
        }
    }

    /// The racer being driven.
    #[must_use]
    pub fn racer(&self) -> &Racer {
        &self.racer
    }

    /// The script's input before frame `frame`.
    fn drive(&mut self, frame: usize) -> Result<(), String> {
        if restart_due(self.racer.race()) {
            self.restart()?;
        }
        for change in scripted(frame) {
            match change {
                Scripted::Camera { follow } => self.set_camera(follow)?,
                Scripted::Livery => livery(),
            }
        }
        self.hold(controls(self.racer.race()));
        Ok(())
    }

    /// Space after three laps: a new race, running. The pause at the
    /// finish let go of every key, so each is pressed again.
    fn restart(&mut self) -> Result<(), String> {
        key(&mut self.racer, "Space", true);
        key(&mut self.racer, "Space", false);
        self.held.clear();
        let (laps, paused) = (self.racer.race().progress.laps, self.racer.paused());
        if laps == 0 && !paused {
            return Ok(());
        }
        Err(format!(
            "Space after three laps left the race at {laps} laps, paused {paused}; \
             it should start a new race, running"
        ))
    }

    /// The camera in the mode named, through the HUD's button when it is
    /// in the other. The racer takes no time in the frame after a switch
    /// (`toggle_camera` clears the last frame's time), so the car stands
    /// still for that frame, which the schedule's model does not count.
    fn set_camera(&mut self, follow: bool) -> Result<(), String> {
        if self.racer.camera().follow != follow {
            self.click_camera_then_track()?;
        }
        let now = self.racer.camera().follow;
        if now != follow {
            return Err(format!(
                "the camera's follow is {now} after the switch to {follow}"
            ));
        }
        if focused(&self.racer) {
            return Err("a HUD element kept focus after a click on the track, \
                        so Space would go to it, not the race"
                .into());
        }
        Ok(())
    }

    /// A click on the camera button, then one on the track, which takes
    /// focus off the button so Space reaches the race.
    fn click_camera_then_track(&mut self) -> Result<(), String> {
        let button = centre_of(&self.racer, "data-action", "camera")?;
        let track = centre_of(&self.racer, "name", "track")?;
        click(&mut self.racer, button);
        click(&mut self.racer, track);
        Ok(())
    }

    /// Presses the keys `controls` needs that are up, releases the rest.
    fn hold(&mut self, controls: Controls) {
        let wanted = keys_for(controls);
        for &code in self.held.difference(&wanted) {
            key(&mut self.racer, code, false);
        }
        for &code in wanted.difference(&self.held) {
            key(&mut self.racer, code, true);
        }
        self.held = wanted;
    }
}

// RELAXED: today's racer has no livery (docs/plans/one-scene.md, 2.4),
// so the scripted change at 30 s changes nothing. R3b's livery enum
// retires this: the change then sets it, and checks that it did.
fn livery() {}

fn keys_for(controls: Controls) -> BTreeSet<&'static str> {
    [
        (controls.throttle > 0.0, "KeyW"),
        (controls.brake > 0.0, "KeyS"),
        (controls.steer < 0.0, "KeyA"),
        (controls.steer > 0.0, "KeyD"),
    ]
    .into_iter()
    .filter_map(|(down, code)| down.then_some(code))
    .collect()
}

#[path = "racer_driven_workload.rs"]
mod workload;
