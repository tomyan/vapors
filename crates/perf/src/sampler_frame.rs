//! One sampled frame (`Sampler::frame`): the change, the app's frame
//! between the GPU clock's marks, the pipeline check, the wait for the
//! GPU, and the phases it recorded.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use vapors_platform::{Frame, Next};

use super::start::BuiltSince;
use super::{FRAME_DEADLINE, INTERVAL, Measured, Sampler};
use crate::frame_counts::{self, Counted, Rise, Start};
use crate::gpu_time::{GpuClock, wait};
use crate::run::Workload;

impl Sampler<'_> {
    /// One frame: the change, then the timed frame between timestamps,
    /// then the untimed wait for the GPU; refused when it built a
    /// pipeline (`sampler_start.rs`). A frame that writes no buffer
    /// (every batch kept, IU4) enters no `write` span; it is recorded as
    /// 0 rather than left out. Every other phase runs in every frame, so
    /// its absence still fails the harness shape test.
    pub(super) fn frame(
        &mut self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        gpu_clock: Option<&mut GpuClock>,
    ) -> Result<Measured, String> {
        let since = self.prepared(workload, index)?;
        let (next, started, took, counted) = self.timed(workload, texture, index, gpu_clock)?;
        since.refuse_any(index)?;
        // frame-to-idle ends here, before the harness's own bookkeeping;
        // gpu_judged_frames.rs holds it at or above the frame's GPU time
        wait(&self.gpu.device, FRAME_DEADLINE)?;
        let idle = started.elapsed();
        workload
            .check(next)
            .map_err(|error| format!("frame {index}: {error}"))?;
        let (phases, counted) = self.taken_phases(counted)?;
        Ok(Measured {
            took,
            idle,
            phases,
            counted,
        })
    }

    /// Frame `index`'s change made and the workload ready, the clock
    /// cleared; the pipeline count from before the change.
    fn prepared(
        &mut self,
        workload: &mut dyn Workload,
        index: usize,
    ) -> Result<BuiltSince, String> {
        let since = BuiltSince::open();
        workload.change(index);
        workload
            .ready()
            .map_err(|error| format!("frame {index}: {error}"))?;
        self.now += INTERVAL;
        let _ = self.clock.take();
        let _ = frame_counts::take(&self.clock)?;
        Ok(since)
    }

    /// The frame's phase times and counts, `write` and `surface`
    /// recorded as 0 when the frame entered none (a page without a
    /// `gpu-surface` has no surface phase).
    fn taken_phases(
        &mut self,
        rise: Rise,
    ) -> Result<(BTreeMap<String, Duration>, Counted), String> {
        let phases = frame_counts::with_maybe_entered(self.clock.take());
        Ok((phases, rise.with_phases(frame_counts::take(&self.clock)?)))
    }

    /// The app's frame, when it started and its wall time, between the
    /// GPU clock's marks.
    fn timed(
        &self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        mut gpu_clock: Option<&mut GpuClock>,
    ) -> Result<(Next, Instant, Duration, Rise), String> {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        if let Some(clock) = gpu_clock.as_deref_mut() {
            clock.mark(device, queue);
        }
        let start = Start::now(&self.clock)?;
        let started = Instant::now();
        let next = workload.app().frame(&self.frame_of(texture, index));
        let took = started.elapsed();
        let counted = start.since(&self.clock)?;
        if let Some(clock) = gpu_clock {
            clock.mark(device, queue);
        }
        Ok((next, started, took, counted))
    }

    pub(super) fn frame_of<'a>(&'a self, texture: &'a wgpu::Texture, index: usize) -> Frame<'a> {
        Frame {
            adapter: &self.gpu.adapter,
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            texture,
            index: index as u64,
            scale_factor: 1.0,
            now: self.now,
        }
    }
}
