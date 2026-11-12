//! The racer's frame, timed: its HUD document over the track's
//! `gpu-surface` at 1100 x 720, paused (the HUD's values unchanged) or
//! racing (its clock and speed change every frame, and the track renders).

use vapors_app::FrameTiming;
use vapors_platform::{App, Next};
use vapors_racer::Racer;

use crate::racer_input::key;
use crate::run::{Counts, Workload};

/// The racer, paused or racing.
pub struct RacerFrames {
    racer: Racer,
    racing: bool,
    /// Frames before this one may still show the change into the state.
    settled_after: usize,
}

/// The frames a paused racer takes to settle (`gpu_frames::settle`: 3
/// or fewer), with room.
const SETTLING: usize = 10;

impl RacerFrames {
    /// A racer, racing when `racing`: Space pressed and released once.
    #[must_use]
    pub fn new(racing: bool) -> Self {
        let mut racer = Racer::new();
        racer.time_frames(true);
        if racing {
            key(&mut racer, "Space", true);
            key(&mut racer, "Space", false);
        }
        Self {
            racer,
            racing,
            settled_after: SETTLING,
        }
    }
}

impl Workload for RacerFrames {
    fn size(&self) -> (u32, u32) {
        (1100, 720)
    }

    fn change(&mut self, _sample: usize) {
        self.settled_after = self.settled_after.saturating_sub(1);
    }

    fn app(&mut self) -> &mut dyn App {
        &mut self.racer
    }

    /// Racing, every frame asks for the next; paused and settled, none does.
    fn check(&self, next: Next) -> Result<(), String> {
        if let Some(error) = self.racer.error() {
            return Err(error);
        }
        match (self.racing, next, self.settled_after) {
            (true, Next::Continue, _) | (false, Next::Wait, _) | (false, _, 1..) => Ok(()),
            (true, other, _) => Err(format!("the race stopped: the frame asked for {other:?}")),
            (false, other, 0) => Err(format!("a paused racer still asks for frames: {other:?}")),
        }
    }

    fn counts(&self) -> Counts {
        racer_counts(&self.racer)
    }

    fn timing(&self) -> FrameTiming {
        self.racer.frame_timing()
    }
}

/// The counts of `racer`'s last frame.
pub(crate) fn racer_counts(racer: &Racer) -> Counts {
    let frame = racer.frame_counts();
    Counts::of(&racer.document().borrow(), frame, racer.render_passes())
}
