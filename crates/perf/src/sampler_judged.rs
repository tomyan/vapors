//! The 60 s run's frames (docs/plans/one-scene.md, 3.2): every frame
//! kept from the first, no warmup and no span clock, each with its GPU
//! time from the app's own timer, read back in the frame after it.

use std::collections::BTreeMap;
use std::time::Duration;

use super::Sampler;
use crate::gpu_reading::frame_gpu_time;
use crate::run::{Counts, Workload};

/// One frame of the run, as it is judged and reported.
#[derive(Clone, Debug, Default)]
pub struct JudgedFrame {
    /// The frame's index from the first.
    pub index: usize,
    /// `App::frame`'s wall time.
    pub cpu: Duration,
    /// The GPU's time in the frame's render passes and surface renders.
    pub gpu: Duration,
    /// From the start of `App::frame` to the return of the GPU wait.
    pub idle: Duration,
    /// Each phase's time; a phase the frame did not enter is absent.
    pub phases: BTreeMap<String, Duration>,
    /// What the frame did, counted.
    pub counts: Counts,
}

impl Sampler<'_> {
    /// `frames` frames of `workload`, every one kept, then one more,
    /// not kept, which reads back the last one's GPU time.
    ///
    /// # Errors
    /// A frame the workload refused, a failed GPU wait, or a frame
    /// without a GPU reading, naming it: a missing reading is never
    /// counted as 0 or left out.
    pub fn judged(
        &mut self,
        workload: &mut dyn Workload,
        frames: usize,
    ) -> Result<Vec<JudgedFrame>, String> {
        let dispatch = tracing::Dispatch::new(self.clock.clone());
        tracing::dispatcher::with_default(&dispatch, || self.judged_frames(workload, frames))
    }

    fn judged_frames(
        &mut self,
        workload: &mut dyn Workload,
        frames: usize,
    ) -> Result<Vec<JudgedFrame>, String> {
        let texture = self.texture(workload.size());
        let mut kept: Vec<JudgedFrame> = Vec::with_capacity(frames);
        for index in 0..=frames {
            let measured = self.frame(workload, &texture, index, None)?;
            if let Some(before) = kept.last_mut() {
                before.gpu = gpu_reading(workload, before.index)?;
            }
            if index < frames {
                kept.push(JudgedFrame {
                    index,
                    cpu: measured.took,
                    gpu: Duration::ZERO,
                    idle: measured.idle,
                    phases: measured.phases,
                    counts: workload.counts(),
                });
            }
        }
        Ok(kept)
    }
}

/// Frame `frame`'s GPU time, read back by the frame after it.
fn gpu_reading(workload: &dyn Workload, frame: usize) -> Result<Duration, String> {
    frame_gpu_time(workload.timing().gpu, frame)?
        .map(|(time, _)| time)
        .ok_or_else(|| {
            format!("frame {frame} has no GPU time: the device has no timestamp queries")
        })
}
