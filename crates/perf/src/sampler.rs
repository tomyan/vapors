//! Runs a workload's frames on a GPU and keeps the timed ones' times.

use std::collections::BTreeMap;
use std::time::Duration;

use vapors_gpu_harness::Gpu;
use vapors_renderer::clear;
use vapors_scene::Color;

use crate::frame_counts::Counted;
use crate::gpu_reading::frame_gpu_time;
use crate::gpu_time::GpuClock;
use crate::run::{Samples, Settings, Workload};
use crate::spans::PhaseClock;

/// The interval between frames' times, as a 60 Hz display's.
pub const INTERVAL: Duration = Duration::from_millis(16);

/// How long the GPU may take to finish a frame. The slowest frame here
/// (the racer on lavapipe) is under a second; ten is far beyond it.
pub(crate) const FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// One frame's measurements: wall time, the time from the start of the
/// app's frame to the return of the GPU wait after it (frame-to-idle),
/// per-phase times, and, with counters, the frame's counts and each
/// phase's.
struct Measured {
    took: Duration,
    idle: Duration,
    phases: BTreeMap<String, Duration>,
    counted: Counted,
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

    /// `workload`'s app started, then `settings.warmup` frames, then
    /// `settings.runs` timed.
    ///
    /// # Errors
    /// `App::start` refusing, a frame that built a pipeline or that the
    /// workload's check refused, or a GPU wait that failed.
    pub fn sample(
        &mut self,
        workload: &mut dyn Workload,
        settings: Settings,
    ) -> Result<Samples, String> {
        self.sample_apps(workload, settings)
    }

    fn sample_apps(
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
        self.begin(workload, texture.format())?;
        let mut samples = Samples {
            warmup: settings.warmup,
            ..Samples::default()
        };
        let timed = settings.warmup..settings.warmup + settings.runs;
        for index in 0..=timed.end {
            let clock = gpu_clock.as_deref_mut().filter(|_| timed.contains(&index));
            let measured = self.frame(workload, &texture, index, clock)?;
            if index > timed.start {
                samples.push_gpu_time(frame_gpu_time(workload.timing().gpu, index - 1)?);
            }
            if timed.contains(&index) {
                samples.push_counts(measured.counted);
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
}

#[path = "sampler_start.rs"]
mod start;

#[path = "sampler_first.rs"]
mod first;
pub use first::{FirstFrame, Phases};

#[path = "sampler_frame.rs"]
mod frame;

#[path = "sampler_judged.rs"]
mod judged;
pub use judged::{JudgedFrame, JudgedRun};

#[path = "sampler_idle.rs"]
mod idle;
