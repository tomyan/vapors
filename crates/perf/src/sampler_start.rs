//! The one place a run starts its app, and the one check that no frame
//! builds a pipeline (one-scene F0: `App::start` builds every one
//! before frame 0), shared by the sampled, judged, idle and first-frame
//! runs.

use std::time::{Duration, Instant};

use vapors_platform::Start;
use vapors_renderer::pipelines::{Built, built};

use super::{FRAME_DEADLINE, Sampler};
use crate::gpu_time::wait;
use crate::run::Workload;

impl Sampler<'_> {
    /// `workload`'s app started on this GPU for textures of `format`,
    /// timed to the GPU's idle after it.
    ///
    /// # Errors
    /// `App::start` refusing, or the GPU wait after it failing.
    pub(super) fn begin(
        &self,
        workload: &mut dyn Workload,
        format: wgpu::TextureFormat,
    ) -> Result<Duration, String> {
        let start = self.start_for(workload.size(), format);
        let started = Instant::now();
        workload
            .app()
            .start(&start)
            .map_err(|error| format!("App::start, before frame 0: {error}"))?;
        wait(&self.gpu.device, FRAME_DEADLINE)?;
        Ok(started.elapsed())
    }

    fn start_for(&self, (width, height): (u32, u32), format: wgpu::TextureFormat) -> Start<'_> {
        Start {
            adapter: &self.gpu.adapter,
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            format,
            width,
            height,
            scale_factor: 1.0,
        }
    }
}

/// The pipelines built since a frame began, read from before its change
/// to after `App::frame`, in every run.
pub(super) struct BuiltSince(Built);

impl BuiltSince {
    /// Opened before the frame's change.
    pub(super) fn open() -> Self {
        Self(built())
    }

    /// Refuses frame `index` when it built a pipeline or shader module.
    pub(super) fn refuse_any(self, index: usize) -> Result<(), String> {
        let since = built().since(self.0);
        if since.is_none() {
            return Ok(());
        }
        Err(format!(
            "frame {index} built {} render pipelines and {} shader modules through \
             vapors_renderer::pipelines; fix: build them in App::start, before frame 0",
            since.pipelines, since.shader_modules
        ))
    }
}
