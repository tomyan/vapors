//! Driving a workload's app as the platform would while nothing happens
//! (`idle.rs`): frames until it settles, then only the frames it asks
//! for, on the app's clock, until the interval is over.

use std::time::Duration;

use vapors_platform::Next;

use super::{FRAME_DEADLINE, INTERVAL, Sampler};
use crate::gpu_time::wait;
use crate::idle::{IdleCounts, SETTLE_FRAMES, Step, did_update, is_settled, step};
use crate::run::Workload;

impl Sampler<'_> {
    /// `workload`'s app left alone for `interval` of its own time after
    /// it settles: what it asked for, counted.
    ///
    /// # Errors
    /// The app not settled within [`SETTLE_FRAMES`], a frame the workload
    /// refused, a failed GPU wait, or the app exiting; each names the fix.
    pub fn idle(
        &mut self,
        workload: &mut dyn Workload,
        interval: Duration,
    ) -> Result<IdleCounts, String> {
        let texture = self.texture(workload.size());
        let mut index = 0;
        let mut next = self.idle_frame(workload, &texture, index)?;
        while !is_settled(next) {
            index += 1;
            if index >= SETTLE_FRAMES {
                return Err(not_settled(index, next));
            }
            self.now += INTERVAL;
            next = self.idle_frame(workload, &texture, index)?;
        }
        let mut counts = IdleCounts {
            settled_after: index + 1,
            ..IdleCounts::default()
        };
        let end = self.now + interval;
        while let Step::Frame(at) = step(next, self.now, end)? {
            self.now = at;
            index += 1;
            next = self.idle_frame(workload, &texture, index)?;
            counts.frames += 1;
            counts.updates += usize::from(did_update(workload.counts().update));
        }
        Ok(counts)
    }

    /// One frame with no change made before it, then the wait for the
    /// GPU, then the workload's check.
    fn idle_frame(
        &mut self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
    ) -> Result<Next, String> {
        workload.change(index);
        workload
            .ready()
            .map_err(|error| format!("idle frame {index}: {error}"))?;
        let next = workload.app().frame(&self.frame_of(texture, index));
        wait(&self.gpu.device, FRAME_DEADLINE)?;
        workload
            .check(next)
            .map_err(|error| format!("idle frame {index}: {error}"))?;
        Ok(next)
    }
}

fn not_settled(frames: usize, last: Next) -> String {
    format!(
        "idle: the app had not settled after {frames} frames with no input (the last asked \
         {last:?}); fix: find what keeps the document changing after the mount \
         (ElementsApp::last_decision names the reason); reproduce: vapors-perf 60 10 --scaling Idle"
    )
}
