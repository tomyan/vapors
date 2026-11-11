//! Sampling a workload: frames of its app after warmup, each timed as a
//! whole and per phase, with the GPU's span where timestamps exist. Between
//! frames, untimed, the sampler waits for the GPU to finish, so a frame's
//! times never include the queue backing up behind the one before.

use std::collections::BTreeMap;
use std::time::Duration;

use vapors_app::FrameCounts;
use vapors_elements::{Document, UpdateCounts};
use vapors_platform::{App, Next};

/// A workload: an app and the change made before each of its frames.
pub trait Workload {
    /// The frame's device size.
    fn size(&self) -> (u32, u32);
    /// The change before frame `sample` (counting warmup), untimed.
    fn change(&mut self, sample: usize);
    /// The app whose frames are timed.
    fn app(&mut self) -> &mut dyn App;
    /// Whether the frame that returned `next` ran as the workload needs.
    ///
    /// # Errors
    /// Why it did not: the app's failure, or a workload's own condition.
    fn check(&self, next: Next) -> Result<(), String>;
    /// The last frame's counts.
    fn counts(&self) -> Counts;
}

/// What the last frame did, counted: the document's update and what the
/// painter handed the GPU.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    /// The update's structural counts.
    pub update: UpdateCounts,
    /// Text layouts the update built.
    pub text_layouts: usize,
    /// The painter's frame.
    pub frame: FrameCounts,
}

impl Counts {
    /// The counts of `document`'s last update and `frame`.
    #[must_use]
    pub fn of(document: &Document, frame: FrameCounts) -> Self {
        Self {
            update: document.update_counts(),
            text_layouts: document.text_layouts_built(),
            frame,
        }
    }
}

/// How many frames to run: `warmup` untimed, then `runs` timed.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Frames run first and not kept.
    pub warmup: usize,
    /// Frames kept.
    pub runs: usize,
}

/// A workload's timed frames.
#[derive(Debug, Default)]
pub struct Samples {
    /// `App::frame`'s wall time.
    pub frame: Vec<Duration>,
    /// Each phase's time per frame, by phase name; a phase a frame did
    /// not enter has 0 for it.
    pub phases: BTreeMap<String, Vec<Duration>>,
    /// The GPU's time from before the frame's first submit to after its
    /// last, per frame; `None` without timestamp queries.
    pub gpu: Option<Vec<Duration>>,
    /// The last frame's counts.
    pub counts: Counts,
}

impl Samples {
    /// Adds one frame's times; phases are aligned by name.
    pub(crate) fn push(&mut self, frame: Duration, phases: BTreeMap<String, Duration>) {
        let earlier = self.frame.len();
        for (phase, time) in phases {
            let times = self.phases.entry(phase).or_default();
            times.resize(earlier, Duration::ZERO);
            times.push(time);
        }
        self.frame.push(frame);
        for times in self.phases.values_mut() {
            times.resize(self.frame.len(), Duration::ZERO);
        }
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
