//! Driving a workload's app as the platform would while nothing happens
//! (`idle.rs`): frames until it settles, then only the frames it asks
//! for, on the app's clock, until the interval is over or the app has
//! asked for more frames than the interval holds.

use std::time::Duration;

use vapors_platform::Next;

use super::{FRAME_DEADLINE, INTERVAL, Sampler};
use crate::gpu_time::wait;
use crate::idle::{IdleCounts, SETTLE_FRAMES, Step, did_update, is_settled, max_frames, step};
use crate::run::Workload;

impl Sampler<'_> {
    /// `workload`'s app left alone for `interval` of its own time after
    /// it settles: what it asked for, counted. `reproduce` is the
    /// command a refusal names.
    ///
    /// # Errors
    /// The app not settled within [`SETTLE_FRAMES`], more frames asked
    /// for than [`max_frames`] (a deadline in the past, asked again), a
    /// frame the workload refused, a failed GPU wait, or the app
    /// exiting; each names the fix and `reproduce`.
    pub fn idle(
        &mut self,
        workload: &mut dyn Workload,
        interval: Duration,
        reproduce: &str,
    ) -> Result<IdleCounts, String> {
        let texture = self.texture(workload.size());
        let mut index = 0;
        let mut next = self.idle_frame(workload, &texture, index)?;
        while !is_settled(next) {
            index += 1;
            if index >= SETTLE_FRAMES {
                return Err(not_settled(index, next, reproduce));
            }
            self.now += INTERVAL;
            next = self.idle_frame(workload, &texture, index)?;
        }
        let mut counts = IdleCounts {
            settled_after: index + 1,
            ..IdleCounts::default()
        };
        let (end, cap) = (self.now + interval, max_frames(interval));
        while let Step::Frame(at) = step(next, self.now, end)? {
            if counts.frames >= cap {
                return Err(over_cap(cap, interval, next, reproduce));
            }
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

fn not_settled(frames: usize, last: Next, reproduce: &str) -> String {
    format!(
        "idle: the app had not settled after {frames} frames with no input (the last asked \
         {last:?}); fix: find what keeps the document changing after the mount \
         (ElementsApp::last_decision names the reason); reproduce: {reproduce}"
    )
}

fn over_cap(cap: usize, interval: Duration, last: Next, reproduce: &str) -> String {
    format!(
        "idle: the app asked for more than {cap} frames in {} s, the most a 60 Hz display shows \
         (the last asked {last:?}: a deadline already passed, asked for again?); fix: return \
         Next::Wait, or a deadline after the frame; reproduce: {reproduce}",
        interval.as_secs()
    )
}
