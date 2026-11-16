//! One sampled frame (`Sampler::frame`): the change, the app's frame
//! between the GPU clock's marks, the pipeline check, the wait for the
//! GPU, and the phases it recorded.

use std::time::{Duration, Instant};

use vapors_platform::{Frame, Next};
use vapors_renderer::WRITE_PHASE;

use super::start::BuiltSince;
use super::{FRAME_DEADLINE, INTERVAL, Measured, PhaseTakes, Sampler};
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
        let (phases, instructions) = self.taken_phases()?;
        Ok(Measured {
            took,
            idle,
            phases,
            instructions: counted.zip(instructions),
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
        let _ = self.clock.take_instructions()?;
        Ok(since)
    }

    /// The frame's phase times and counts, `write` recorded as 0 when
    /// the frame entered none.
    fn taken_phases(&mut self) -> Result<PhaseTakes, String> {
        let mut phases = self.clock.take();
        let mut instructions = self.clock.take_instructions()?;
        phases.entry(WRITE_PHASE.into()).or_default();
        if let Some(counts) = &mut instructions {
            counts.entry(WRITE_PHASE.into()).or_default();
        }
        Ok((phases, instructions))
    }

    /// The app's frame, when it started and its wall time, between the
    /// GPU clock's marks.
    fn timed(
        &self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        mut gpu_clock: Option<&mut GpuClock>,
    ) -> Result<(Next, Instant, Duration, Option<u64>), String> {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        if let Some(clock) = gpu_clock.as_deref_mut() {
            clock.mark(device, queue);
        }
        let before = self.clock.instructions_now()?;
        let started = Instant::now();
        let next = workload.app().frame(&self.frame_of(texture, index));
        let took = started.elapsed();
        let counted = before
            .zip(self.clock.instructions_now()?)
            .map(|(a, b)| b - a);
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
