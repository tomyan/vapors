//! The 60 s run's frames (docs/plans/one-scene.md, 3.2): every frame
//! kept from the first, no warmup and no span clock, each with its GPU
//! time from the app's own timer, read back in the frame after it.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use vapors_platform::Start;
use vapors_renderer::pipelines::{Built, built};

use super::{FRAME_DEADLINE, Measured, Sampler};
use crate::gpu_reading::frame_gpu_time;
use crate::gpu_time::wait;
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

/// A judged run: the app's start, before frame 0 and not judged, and
/// its frames.
#[derive(Debug)]
pub struct JudgedRun {
    /// `App::start`'s wall time, to the GPU's idle after it.
    pub start_up: Duration,
    /// Every frame, from the first.
    pub frames: Vec<JudgedFrame>,
}

impl Sampler<'_> {
    /// `workload`'s app started, then `frames` frames of it, every one
    /// kept, then one more, not kept, which reads back the last one's
    /// GPU time.
    ///
    /// # Errors
    /// A start the app refused; a frame the workload refused, a frame
    /// that built a pipeline or shader module, a failed GPU wait, or a
    /// frame without a GPU reading, naming it: a missing reading is
    /// never counted as 0 or left out.
    pub fn judged(
        &mut self,
        workload: &mut dyn Workload,
        frames: usize,
    ) -> Result<JudgedRun, String> {
        let dispatch = tracing::Dispatch::new(self.clock.clone());
        tracing::dispatcher::with_default(&dispatch, || self.judged_frames(workload, frames))
    }

    fn judged_frames(
        &mut self,
        workload: &mut dyn Workload,
        frames: usize,
    ) -> Result<JudgedRun, String> {
        if frames == 0 {
            return Err("a judged run of 0 frames judges nothing; give it at least one".into());
        }
        let texture = self.texture(workload.size());
        let start_up = self.start(workload, texture.format())?;
        let (mut kept, mut waiting) = (Vec::with_capacity(frames), None);
        for index in 0..=frames {
            let before = built();
            let measured = self.frame(workload, &texture, index, None)?;
            nothing_built(index, built().since(before))?;
            if let Some((before, counts)) = waiting.take() {
                kept.push(judged(
                    before,
                    &counts,
                    gpu_reading(workload, index - 1)?,
                    index - 1,
                ));
            }
            if index < frames {
                waiting = Some((measured, workload.counts()));
            }
        }
        Ok(JudgedRun {
            start_up,
            frames: kept,
        })
    }

    /// The app's start on this GPU for textures of `format`, timed to
    /// the GPU's idle after it.
    fn start(
        &self,
        workload: &mut dyn Workload,
        format: wgpu::TextureFormat,
    ) -> Result<Duration, String> {
        let (width, height) = workload.size();
        let start = Start {
            adapter: &self.gpu.adapter,
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            format,
            width,
            height,
            scale_factor: 1.0,
        };
        let started = Instant::now();
        let app = workload.app();
        app.start(&start)
            .map_err(|error| format!("App::start, before frame 0: {error}"))?;
        wait(&self.gpu.device, FRAME_DEADLINE)?;
        Ok(started.elapsed())
    }
}

/// Refuses frame `index` when it built a pipeline or shader module:
/// every one is built in `App::start`, before frame 0 (one-scene F0).
fn nothing_built(index: usize, built: Built) -> Result<(), String> {
    if built.is_none() {
        return Ok(());
    }
    Err(format!(
        "frame {index} built {} render pipelines and {} shader modules through \
         vapors_renderer::pipelines; fix: build them in App::start, before frame 0",
        built.pipelines, built.shader_modules
    ))
}

/// Frame `index`, kept once its GPU reading `gpu` has come back: a
/// frame is never kept without one.
fn judged(measured: Measured, counts: &Counts, gpu: Duration, index: usize) -> JudgedFrame {
    JudgedFrame {
        index,
        cpu: measured.took,
        gpu,
        idle: measured.idle,
        phases: measured.phases,
        counts: *counts,
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
