//! GPU tier: the 60 s run's frames, not timed against a budget: every
//! frame kept from the first, each with a GPU reading and a
//! frame-to-idle time no shorter than its CPU time.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::racer_driven::DrivenRacer;
use vapors_perf::sampler::Sampler;

#[test]
fn gpu_every_judged_frame_from_the_first_has_a_gpu_time_and_its_time_to_idle() {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut driven = DrivenRacer::new();

    // When
    let frames = Sampler::new(&gpu)
        .judged(&mut driven, 30)
        .expect("every frame ran and was read back");

    // Then
    let indices: Vec<usize> = frames.iter().map(|f| f.index).collect();
    assert_eq!(indices, (0..30).collect::<Vec<_>>());
    for frame in &frames {
        assert!(
            !frame.gpu.is_zero(),
            "frame {} has no GPU time",
            frame.index
        );
        assert!(frame.idle >= frame.cpu, "frame {}: {frame:?}", frame.index);
        assert!(
            frame.counts.frame.total().draw_calls > 0,
            "frame {}",
            frame.index
        );
    }
}
