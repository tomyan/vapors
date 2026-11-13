//! Fake apps and workloads for the idle measurement's GPU tests
//! (`gpu_idle_frames.rs`): apps that answer a fixed `Next`, and the rows
//! page driven by deadlines after it settles.

use std::time::Duration;

use vapors_app::FrameTiming;
use vapors_perf::rows::Rows;
use vapors_perf::run::{Counts, Workload};
use vapors_platform::{App, Event, Frame, Next, Platform};

/// What a fake app answers after each frame, drawing nothing.
pub enum Answers {
    Always(Next),
    /// A deadline 100 ms on from the first frame, then `Always`.
    DeadlineThen(Next),
    /// A deadline at the frame's own time, every frame.
    DeadlineNow,
}

impl App for Answers {
    fn event(&mut self, _: Event, _: &Platform) {}

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match *self {
            Self::Always(next) => next,
            Self::DeadlineThen(next) => {
                *self = Self::Always(next);
                Next::WaitUntil(frame.now + Duration::from_millis(100))
            }
            Self::DeadlineNow => Next::WaitUntil(frame.now),
        }
    }
}

/// A workload around a fake app: no change, no counts, no GPU time.
pub struct Fake(pub Answers);

impl Workload for Fake {
    fn size(&self) -> (u32, u32) {
        (64, 64)
    }

    fn change(&mut self, _: usize) {}

    fn app(&mut self) -> &mut dyn App {
        &mut self.0
    }

    fn check(&self, _: Next) -> Result<(), String> {
        Ok(())
    }

    fn counts(&self) -> Counts {
        Counts::default()
    }

    fn timing(&self) -> FrameTiming {
        unreachable!("the idle driver reads no frame timing")
    }

    fn times_gpu(&self) -> bool {
        false
    }
}

/// The rows page whose app asks for a frame 100 ms after any it would
/// have waited on, so the driver runs frames after settling; its change
/// is made only from the third frame, after the page has settled.
pub struct Deadlines(Rows);

impl Deadlines {
    pub fn new(rows: Rows) -> Self {
        Self(rows)
    }
}

/// Frames before this are the settling ones; no change is made before them.
const EDIT_FROM: usize = 2;

impl App for Deadlines {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.0.app().event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        match self.0.app().frame(frame) {
            Next::Wait => Next::WaitUntil(frame.now + Duration::from_millis(100)),
            other => other,
        }
    }
}

impl Workload for Deadlines {
    fn size(&self) -> (u32, u32) {
        self.0.size()
    }

    fn change(&mut self, sample: usize) {
        if sample >= EDIT_FROM {
            self.0.change(sample);
        } else {
            self.0.take_ax_update();
        }
    }

    fn app(&mut self) -> &mut dyn App {
        self
    }

    fn check(&self, next: Next) -> Result<(), String> {
        self.0.check(next)
    }

    fn counts(&self) -> Counts {
        self.0.counts()
    }

    fn timing(&self) -> FrameTiming {
        self.0.timing()
    }
}
