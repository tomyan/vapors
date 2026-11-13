//! Sampling a workload: frames of its app after warmup, each timed as a
//! whole and per phase, with the GPU's span where timestamps exist. Between
//! frames, untimed, the sampler waits for the GPU to finish, so a frame's
//! times never include the queue backing up behind the one before.

use std::collections::BTreeMap;
use std::time::Duration;

use vapors_app::{FrameCounts, FrameTiming, GpuResolution};
use vapors_elements::{Document, UpdateCounts};
use vapors_platform::{App, Next};

use crate::instruction_samples::InstructionSamples;

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
    /// The app's frame times, its GPU time read back by this frame.
    fn timing(&self) -> FrameTiming;
    /// Whether the workload can run frame `sample`'s app: called after
    /// `change`, before the frame.
    ///
    /// # Errors
    /// Why not (a page that failed to mount, with its cause).
    fn ready(&self) -> Result<(), String> {
        Ok(())
    }
    /// Whether the app reads each frame's GPU time back in the frame after
    /// it. A workload that makes a new app for every frame has no such
    /// frame, and says so here; the sampler then keeps no GPU time.
    fn times_gpu(&self) -> bool {
        true
    }
    /// The kinds of frame the change alternates between, in the order
    /// it makes them from frame 0 (warmup counted): the full run
    /// reports each apart (`frame_kinds.rs`). One kind by default.
    fn frame_kinds(&self) -> &'static [&'static str] {
        &["every"]
    }
}

/// What the last frame did, counted: the document's update and what the
/// painter handed the GPU.
#[derive(Clone, Copy, Debug, Default)]
pub struct Counts {
    /// The update's structural counts.
    pub update: UpdateCounts,
    /// Text layouts the update built.
    pub text_layouts: usize,
    /// Runs the update shaped (each layout is broken from one).
    pub texts_shaped: usize,
    /// The painter's frame.
    pub frame: FrameCounts,
    /// Render passes the frame's encoder began, flushes included.
    pub render_passes: u64,
}

impl Counts {
    /// The counts of `document`'s last update, `frame` and the
    /// `render_passes` it began.
    #[must_use]
    pub fn of(document: &Document, frame: FrameCounts, render_passes: u64) -> Self {
        Self {
            update: document.update_counts(),
            text_layouts: document.text_layouts_built(),
            texts_shaped: document.texts_shaped(),
            frame,
            render_passes,
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
    /// last, per frame: an upper bound, as it holds the GPU's idle gaps;
    /// `None` without timestamp queries.
    pub gpu: Option<Vec<Duration>>,
    /// The GPU's time in the frame's render passes and `gpu-surface`
    /// renders, from the app's `FrameTiming`, per frame; `None` without
    /// timestamp queries.
    pub gpu_time: Option<Vec<Duration>>,
    /// The smallest step the GPU times can show; `None` without them.
    pub gpu_resolution: Option<GpuResolution>,
    /// Instructions per frame; `None` unless the sampler counts them.
    pub instructions: Option<InstructionSamples>,
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

    /// Adds one frame's GPU time and its resolution; `None`, without
    /// timestamp queries, adds none.
    pub(crate) fn push_gpu_time(&mut self, time: Option<(Duration, GpuResolution)>) {
        if let Some((time, resolution)) = time {
            self.gpu_time.get_or_insert_default().push(time);
            self.gpu_resolution = Some(resolution);
        }
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
