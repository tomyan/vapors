//! GPU tier: the 60 s run's frames, not timed against a budget: every
//! frame kept from the first, each with a GPU reading and a
//! frame-to-idle time no shorter than its GPU time; and no frame that
//! builds a pipeline (one-scene F0).

use vapors_app::FrameTiming;
use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::racer_driven::DrivenRacer;
use vapors_perf::run::{Counts, Workload};
use vapors_perf::sampler::Sampler;
use vapors_platform::{App, Next};
use vapors_renderer::SolidPipeline;

#[test]
fn gpu_every_judged_frame_from_the_first_has_a_gpu_time_and_its_time_to_idle() {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut driven = DrivenRacer::new();

    // When
    let frames = Sampler::new(&gpu)
        .judged(&mut driven, 30)
        .expect("every frame ran and was read back")
        .frames;

    // Then
    let indices: Vec<usize> = frames.iter().map(|f| f.index).collect();
    assert_eq!(indices, (0..30).collect::<Vec<_>>());
    for frame in &frames {
        assert!(
            !frame.gpu.is_zero(),
            "frame {} has no GPU time",
            frame.index
        );
        // the frame's passes run after its start and end before the wait
        // returns, so frame-to-idle that ends at the wait covers them
        assert!(frame.idle >= frame.gpu, "frame {}: {frame:?}", frame.index);
        assert!(
            frame.counts.frame.total().draw_calls > 0,
            "frame {}",
            frame.index
        );
    }
}

#[test]
fn gpu_a_judged_run_of_no_frames_is_refused() {
    let gpu = gpu_with(FEATURES);
    let error = Sampler::new(&gpu)
        .judged(&mut DrivenRacer::new(), 0)
        .expect_err("nothing to judge");
    assert!(error.contains("0 frames"), "{error}");
}

/// The driven racer, but its input before frame `at` builds a pipeline.
struct BuildsAPipeline {
    driven: DrivenRacer,
    device: wgpu::Device,
    at: usize,
}

impl Workload for BuildsAPipeline {
    fn size(&self) -> (u32, u32) {
        self.driven.size()
    }
    fn change(&mut self, sample: usize) {
        if sample == self.at {
            let _late = SolidPipeline::new(&self.device, wgpu::TextureFormat::Rgba8Unorm);
        }
        self.driven.change(sample);
    }
    fn app(&mut self) -> &mut dyn App {
        self.driven.app()
    }
    fn ready(&self) -> Result<(), String> {
        self.driven.ready()
    }
    fn check(&self, next: Next) -> Result<(), String> {
        self.driven.check(next)
    }
    fn counts(&self) -> Counts {
        self.driven.counts()
    }
    fn timing(&self) -> FrameTiming {
        self.driven.timing()
    }
}

#[test]
fn gpu_a_judged_frame_that_builds_a_pipeline_is_refused_naming_it() {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut late = BuildsAPipeline {
        driven: DrivenRacer::new(),
        device: gpu.device.clone(),
        at: 3,
    };

    // When
    let error = Sampler::new(&gpu)
        .judged(&mut late, 10)
        .expect_err("frame 3 built a pipeline");

    // Then
    assert!(
        error.starts_with("frame 3 built 1 render pipelines and 1 shader modules"),
        "{error}"
    );
}

#[test]
fn gpu_the_driven_racer_builds_every_pipeline_before_frame_0() {
    // Given
    let gpu = gpu_with(FEATURES);

    // When
    let run = Sampler::new(&gpu).judged(&mut DrivenRacer::new(), 60);

    // Then
    let run = run.expect("no frame built a pipeline");
    assert!(!run.start_up.is_zero());
}
