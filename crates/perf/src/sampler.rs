//! Runs a workload's frames on a GPU and keeps the timed ones' times.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use vapors_gpu_harness::Gpu;
use vapors_platform::{Frame, Next};
use vapors_renderer::clear;
use vapors_scene::Color;

use crate::gpu_time::{GpuClock, wait};
use crate::run::{Samples, Settings, Workload};
use crate::spans::PhaseClock;

/// The interval between frames' times, as a 60 Hz display's.
const INTERVAL: Duration = Duration::from_millis(16);

/// How long the GPU may take to finish a frame. The slowest frame here
/// (the racer on lavapipe) is under a second; ten is far beyond it.
const FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// A GPU and the phase clock frames are timed by.
pub struct Sampler<'g> {
    gpu: &'g Gpu,
    clock: PhaseClock,
    now: vapors_platform::Instant,
}

impl<'g> Sampler<'g> {
    /// A sampler on `gpu`.
    #[must_use]
    pub fn new(gpu: &'g Gpu) -> Self {
        Self {
            gpu,
            clock: PhaseClock::default(),
            now: vapors_platform::Instant::now(),
        }
    }

    /// `settings.warmup` frames of `workload`, then `settings.runs` timed.
    ///
    /// # Errors
    /// A frame the workload's check refused, or a GPU wait that failed.
    pub fn sample(
        &mut self,
        workload: &mut dyn Workload,
        settings: Settings,
    ) -> Result<Samples, String> {
        let dispatch = tracing::Dispatch::new(self.clock.clone());
        tracing::dispatcher::with_default(&dispatch, || self.frames(workload, settings))
    }

    fn frames(
        &mut self,
        workload: &mut dyn Workload,
        settings: Settings,
    ) -> Result<Samples, String> {
        let (width, height) = workload.size();
        let texture = clear(
            &self.gpu.device,
            &self.gpu.queue,
            width,
            height,
            Color::default(),
        );
        let frames = u32::try_from(settings.runs).map_err(|_| "too many runs".to_owned())?;
        let mut gpu_clock = GpuClock::new(&self.gpu.device, frames);
        let mut samples = Samples::default();
        for index in 0..settings.warmup + settings.runs {
            let timed = index >= settings.warmup;
            let clock = gpu_clock.as_mut().filter(|_| timed);
            let (frame, phases) = self.frame(workload, &texture, index, clock)?;
            if timed {
                samples.push(frame, phases);
            }
        }
        samples.gpu = gpu_clock
            .map(|clock| clock.spans(&self.gpu.device, &self.gpu.queue))
            .transpose()?;
        samples.counts = workload.counts();
        Ok(samples)
    }

    /// One frame: the change, then the timed frame between timestamps,
    /// then the untimed wait for the GPU.
    fn frame(
        &mut self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        gpu_clock: Option<&mut GpuClock>,
    ) -> Result<(Duration, BTreeMap<String, Duration>), String> {
        workload.change(index);
        self.now += INTERVAL;
        let _ = self.clock.take();
        let (next, took) = self.timed(workload, texture, index, gpu_clock);
        let phases = self.clock.take();
        workload
            .check(next)
            .map_err(|error| format!("frame {index}: {error}"))?;
        wait(&self.gpu.device, FRAME_DEADLINE)?;
        Ok((took, phases))
    }

    /// The app's frame, its wall time, between the GPU clock's marks.
    fn timed(
        &self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        mut gpu_clock: Option<&mut GpuClock>,
    ) -> (Next, Duration) {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        if let Some(clock) = gpu_clock.as_deref_mut() {
            clock.mark(device, queue);
        }
        let started = Instant::now();
        let next = workload.app().frame(&self.frame_of(texture, index));
        let took = started.elapsed();
        if let Some(clock) = gpu_clock {
            clock.mark(device, queue);
        }
        (next, took)
    }

    fn frame_of<'a>(&'a self, texture: &'a wgpu::Texture, index: usize) -> Frame<'a> {
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
