//! GPU tier: the one-scene example as the judged run's workload,
//! without timing: driven past its first click, every frame drawn as its
//! schedule says and asking for the next (the workload's check refuses
//! a frame that is not).

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::one_scene_driven::DrivenScene;
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

#[test]
fn gpu_the_driven_scene_runs_past_its_first_click_each_frame_confirmed() {
    // Given: 130 frames, the click on light 1 at frame 125
    let mut driven = DrivenScene::new().expect("mounts");
    let gpu = gpu_with(FEATURES);

    // When
    let samples = Sampler::new(&gpu)
        .sample(
            &mut driven,
            Settings {
                warmup: 0,
                runs: 130,
            },
        )
        .expect("every frame confirmed and asked for the next");

    // Then
    assert_eq!(samples.frame.len(), 130);
}
