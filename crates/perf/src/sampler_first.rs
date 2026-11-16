//! A page's first frame as a user waits for it (property 7; review of
//! #428, H1): `App::start` timed to the GPU's idle, then frame 0 timed
//! to submit and to the GPU's idle, as the platform runs them
//! (`handler_gpu.rs` starts the app before frame 0). Each part's phases
//! are kept apart: `App::start` holds the first update, so its style
//! and layout are there, not in frame 0.

use std::collections::BTreeMap;
use std::time::Duration;

use super::Sampler;
use crate::run::Workload;

/// Times, and with a counter instructions, by phase name.
pub type Phases = (BTreeMap<String, Duration>, Option<BTreeMap<String, u64>>);

/// One page's start and first frame.
#[derive(Debug)]
pub struct FirstFrame {
    /// `App::start` to the GPU's idle after it.
    pub start: Duration,
    /// The phases `App::start` entered.
    pub start_phases: Phases,
    /// Frame 0's wall time to `submit` returning.
    pub frame_cpu: Duration,
    /// Frame 0 from its start to the GPU's idle.
    pub frame_idle: Duration,
    /// Frame 0's phases.
    pub frame_phases: Phases,
    /// Frame 0's instructions, with a counter.
    pub frame_instructions: Option<u64>,
}

impl Sampler<'_> {
    /// `workload`'s app started and its frame 0 drawn, each timed.
    ///
    /// # Errors
    /// The workload not ready (a failed mount, with its cause),
    /// `App::start` refusing, frame 0 building a pipeline or refused by
    /// the workload's check, or a GPU wait past its deadline.
    pub fn first_frame(&mut self, workload: &mut dyn Workload) -> Result<FirstFrame, String> {
        let dispatch = tracing::Dispatch::new(self.clock.clone());
        tracing::dispatcher::with_default(&dispatch, || self.started_frame(workload))
    }

    fn started_frame(&mut self, workload: &mut dyn Workload) -> Result<FirstFrame, String> {
        workload.ready()?;
        let texture = self.texture(workload.size());
        let _ = (self.clock.take(), self.clock.take_instructions()?);
        let start = self.begin(workload, texture.format())?;
        let start_phases = (self.clock.take(), self.clock.take_instructions()?);
        let frame = self.frame(workload, &texture, 0, None)?;
        let (frame_instructions, frame_counts) = frame.instructions.unzip();
        Ok(FirstFrame {
            start,
            start_phases,
            frame_cpu: frame.took,
            frame_idle: frame.idle,
            frame_phases: (frame.phases, frame_counts),
            frame_instructions,
        })
    }
}
