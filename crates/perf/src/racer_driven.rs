//! The racer driven by the script (`driver.rs`) through a player's
//! input: the controls as held keys, Space to start again after three
//! laps, and a click on the HUD's camera button. Each scripted change
//! is checked to have happened; one that did not ends the run.

use std::collections::BTreeSet;

use vapors_app::FrameTiming;
use vapors_platform::{App, Next};
use vapors_racer::Racer;
use vapors_racer::model::Controls;

use crate::driver::{Scripted, controls, restart_due, scripted};
use crate::racer::racer_counts;
use crate::racer_input::{centre_of, click, focused, key};
use crate::run::{Counts, Workload};

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
                Scripted::CameraSwitch => self.switch_camera()?,
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
        match self.racer.race().progress.laps {
            0 => Ok(()),
            laps => Err(format!(
                "Space after three laps left the race at {laps} laps; it should start a new race"
            )),
        }
    }

    /// A click on the HUD's camera button, then one on the track, which
    /// takes focus off the button so Space reaches the race.
    fn switch_camera(&mut self) -> Result<(), String> {
        let follow = self.racer.camera().follow;
        let (button, track) = (
            centre_of(&self.racer, "data-action", "camera")?,
            centre_of(&self.racer, "name", "track")?,
        );
        click(&mut self.racer, button);
        click(&mut self.racer, track);
        if self.racer.camera().follow == follow {
            return Err(format!(
                "a click on the camera button left the camera's follow at {follow}"
            ));
        }
        if focused(&self.racer) {
            return Err("a HUD element kept focus after a click on the track, \
                        so Space would go to it, not the race"
                .into());
        }
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

impl Workload for DrivenRacer {
    fn size(&self) -> (u32, u32) {
        (1100, 720)
    }

    fn change(&mut self, sample: usize) {
        if self.fault.is_none() {
            self.fault = self.drive(sample).err();
        }
    }

    fn app(&mut self) -> &mut dyn App {
        &mut self.racer
    }

    fn ready(&self) -> Result<(), String> {
        self.fault.clone().map_or(Ok(()), Err)
    }

    /// Every frame asks for the next, but the one that finished the
    /// race: the racer pauses there, and the next frame's input starts
    /// it again.
    fn check(&self, next: Next) -> Result<(), String> {
        if let Some(error) = self.racer.error() {
            return Err(error);
        }
        match next {
            Next::Continue => Ok(()),
            Next::Wait if restart_due(self.racer.race()) => Ok(()),
            other => Err(format!(
                "the driven race stopped: the frame asked for {other:?}"
            )),
        }
    }

    fn counts(&self) -> Counts {
        racer_counts(&self.racer)
    }

    fn timing(&self) -> FrameTiming {
        self.racer.frame_timing()
    }
}
