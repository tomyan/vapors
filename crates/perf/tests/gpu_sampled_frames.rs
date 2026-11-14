//! GPU tier: the sampled frames `--scaling` and the baseline count start
//! after `App::start`, which builds every pipeline (one-scene F0), and a
//! sampled or idle frame that builds one, in its change or its frame, is
//! refused, naming it: one check, one window, for every run.

use vapors_app::FrameTiming;
use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::{Counts, Settings, Workload};
use vapors_perf::sampler::Sampler;
use vapors_platform::{App, Next};
use vapors_renderer::SolidPipeline;

const FEW: Settings = Settings { warmup: 1, runs: 3 };

/// The rows page, building a pipeline before frame `at`.
struct BuildsAPipeline {
    rows: Rows,
    device: wgpu::Device,
    at: usize,
}

impl Workload for BuildsAPipeline {
    fn size(&self) -> (u32, u32) {
        self.rows.size()
    }
    fn change(&mut self, sample: usize) {
        if sample == self.at {
            let _late = SolidPipeline::new(&self.device, wgpu::TextureFormat::Rgba8Unorm);
        }
        self.rows.change(sample);
    }
    fn app(&mut self) -> &mut dyn App {
        self.rows.app()
    }
    fn ready(&self) -> Result<(), String> {
        self.rows.ready()
    }
    fn check(&self, next: Next) -> Result<(), String> {
        self.rows.check(next)
    }
    fn counts(&self) -> Counts {
        self.rows.counts()
    }
    fn timing(&self) -> FrameTiming {
        self.rows.timing()
    }
}

#[test]
fn gpu_the_rows_pages_sampled_frames_build_no_pipeline() {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut rows = Rows::new(100, RowChange::Insert);

    // When
    let samples = Sampler::new(&gpu).sample(&mut rows, FEW);

    // Then
    samples.expect("App::start built every pipeline before frame 0");
}

#[test]
fn gpu_a_sampled_frame_that_builds_a_pipeline_is_refused_naming_it() {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut late = BuildsAPipeline {
        rows: Rows::new(100, RowChange::Insert),
        device: gpu.device.clone(),
        at: 2,
    };

    // When
    let error = Sampler::new(&gpu)
        .sample(&mut late, FEW)
        .expect_err("frame 2 built a pipeline");

    // Then
    assert!(
        error.starts_with("frame 2 built 1 render pipelines and 1 shader modules"),
        "{error}"
    );
}

#[test]
fn gpu_an_idle_frame_whose_change_builds_a_pipeline_is_refused_naming_it() {
    // Given: the rows page left alone, a pipeline built before frame 1
    let gpu = gpu_with(FEATURES);
    let mut late = BuildsAPipeline {
        rows: Rows::new(100, RowChange::Idle),
        device: gpu.device.clone(),
        at: 1,
    };

    // When
    let error = Sampler::new(&gpu)
        .idle(&mut late, std::time::Duration::from_secs(1), "the test")
        .expect_err("frame 1 built a pipeline");

    // Then
    assert!(
        error.starts_with("idle frame 1 built 1 render pipelines and 1 shader modules"),
        "{error}"
    );
}
