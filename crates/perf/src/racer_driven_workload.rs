//! The driven racer as a workload: the script's input before each
//! frame, and each frame checked to race under the keys sent.

use vapors_app::FrameTiming;
use vapors_platform::{App, Next};

use super::{DrivenRacer, keys_for};
use crate::driver::restart_due;
use crate::racer::racer_counts;
use crate::run::{Counts, Workload};

impl DrivenRacer {
    /// Whether the racer races under the keys sent: not paused, and its
    /// controls those of the keys the script holds.
    fn racing(&self) -> Result<(), String> {
        if self.racer.paused() {
            return Err("the racer is paused mid-race".into());
        }
        let holds = keys_for(self.racer.controls());
        if holds != self.held {
            return Err(format!(
                "the racer's controls are those of {holds:?}; the script holds {:?}",
                self.held
            ));
        }
        Ok(())
    }
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

    /// Every frame races under the keys sent and asks for the next, but
    /// the one that finished the race: the racer pauses there and lets
    /// go of the keys, and the next frame's input starts it again.
    fn check(&self, next: Next) -> Result<(), String> {
        if let Some(error) = self.racer.error() {
            return Err(error);
        }
        if restart_due(self.racer.race()) {
            return Ok(());
        }
        self.racing()?;
        match next {
            Next::Continue => Ok(()),
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
