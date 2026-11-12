//! Runs a workload's frames on a GPU and keeps the timed ones' times.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use vapors_gpu_harness::Gpu;
use vapors_platform::{Frame, Next};
use vapors_renderer::clear;
use vapors_scene::Color;

use crate::gpu_reading::frame_gpu_time;
use crate::gpu_time::{GpuClock, wait};
use crate::instruction_samples::InstructionSamples;
use crate::run::{Samples, Settings, Workload};
use crate::spans::PhaseClock;

/// The interval between frames' times, as a 60 Hz display's.
const INTERVAL: Duration = Duration::from_millis(16);

/// How long the GPU may take to finish a frame. The slowest frame here
/// (the racer on lavapipe) is under a second; ten is far beyond it.
const FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// One frame's measurements: wall time and per-phase times, and, with a
/// counter, the frame's instructions and each phase's.
struct Measured {
    took: Duration,
    phases: BTreeMap<String, Duration>,
    instructions: Option<(u64, BTreeMap<String, u64>)>,
}

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
        Self::with_clock(gpu, PhaseClock::default())
    }

    /// A sampler on `gpu` timing by `clock`, which counts instructions
    /// too if it was made to.
    #[must_use]
    pub fn with_clock(gpu: &'g Gpu, clock: PhaseClock) -> Self {
        Self {
            gpu,
            clock,
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
        let frames = u32::try_from(settings.runs).map_err(|_| "too many runs".to_owned())?;
        let mut gpu_clock = GpuClock::new(&self.gpu.device, frames);
        let mut samples = self.timed_frames(workload, settings, gpu_clock.as_mut())?;
        samples.gpu = gpu_clock
            .map(|clock| clock.spans(&self.gpu.device, &self.gpu.queue))
            .transpose()?;
        Ok(samples)
    }

    /// The warmup frames, then the timed ones, kept with the last one's
    /// counts, on one texture, then one more, untimed, which reads back
    /// the last one's GPU time.
    fn timed_frames(
        &mut self,
        workload: &mut dyn Workload,
        settings: Settings,
        mut gpu_clock: Option<&mut GpuClock>,
    ) -> Result<Samples, String> {
        let texture = self.texture(workload.size());
        let mut samples = Samples::default();
        let timed = settings.warmup..settings.warmup + settings.runs;
        for index in 0..=timed.end {
            let clock = gpu_clock.as_deref_mut().filter(|_| timed.contains(&index));
            let measured = self.frame(workload, &texture, index, clock)?;
            if index > timed.start && workload.times_gpu() {
                samples.push_gpu_time(frame_gpu_time(workload.timing().gpu, index - 1)?);
            }
            if timed.contains(&index) {
                if let Some((count, phases)) = measured.instructions {
                    let kept = samples
                        .instructions
                        .get_or_insert_with(InstructionSamples::default);
                    kept.push(count, phases);
                }
                samples.push(measured.took, measured.phases);
                samples.counts = workload.counts();
            }
        }
        Ok(samples)
    }

    fn texture(&self, (width, height): (u32, u32)) -> wgpu::Texture {
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        clear(device, queue, width, height, Color::default())
    }

    /// One frame: the change, then the timed frame between timestamps,
    /// then the untimed wait for the GPU.
    fn frame(
        &mut self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        gpu_clock: Option<&mut GpuClock>,
    ) -> Result<Measured, String> {
        workload.change(index);
        self.now += INTERVAL;
        let _ = self.clock.take();
        let _ = self.clock.take_instructions()?;
        let (next, took, frame_instructions) = self.timed(workload, texture, index, gpu_clock)?;
        let phases = self.clock.take();
        let instructions = self.clock.take_instructions()?;
        workload
            .check(next)
            .map_err(|error| format!("frame {index}: {error}"))?;
        wait(&self.gpu.device, FRAME_DEADLINE)?;
        let instructions = frame_instructions.zip(instructions);
        Ok(Measured {
            took,
            phases,
            instructions,
        })
    }

    /// The app's frame, its wall time, between the GPU clock's marks.
    fn timed(
        &self,
        workload: &mut dyn Workload,
        texture: &wgpu::Texture,
        index: usize,
        mut gpu_clock: Option<&mut GpuClock>,
    ) -> Result<(Next, Duration, Option<u64>), String> {
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
        Ok((next, took, counted))
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
