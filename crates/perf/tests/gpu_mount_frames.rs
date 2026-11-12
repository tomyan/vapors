//! GPU tier: the Mount workload runs through the sampler, every frame
//! the first of a freshly mounted page, with no GPU time read back and
//! a mount measured for each page.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::instructions_check::checked_counter;
use vapors_perf::mount::Mount;
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

const SETTINGS: Settings = Settings { warmup: 1, runs: 3 };
const ROWS: usize = 300;

#[test]
fn gpu_mount_frames_each_draw_a_fresh_page_and_keep_no_gpu_time() {
    // Given
    let mut mount = Mount::new(ROWS, checked_counter().expect("perf events"));
    let gpu = gpu_with(FEATURES);

    // When
    let samples = Sampler::new(&gpu)
        .sample(&mut mount, SETTINGS)
        .expect("the frames ran");

    // Then
    assert_eq!(samples.frame.len(), SETTINGS.runs);
    assert!(samples.gpu_time.is_none());
    assert!(samples.counts.update.elements_restyled >= ROWS * 4);
    assert_eq!(mount.mounted.len(), SETTINGS.warmup + SETTINGS.runs + 1);
}

#[test]
fn gpu_a_failed_mount_ends_the_sample_with_its_cause_not_a_panic() {
    // Given
    let mut mount = Mount::new(10, checked_counter().expect("perf events"));
    mount.fail_for_test("reading /proc/self/status: gone");
    let gpu = gpu_with(FEATURES);

    // When
    let result = Sampler::new(&gpu).sample(&mut mount, SETTINGS);

    // Then
    let error = result.expect_err("the mount failed");
    assert!(error.contains("reading /proc/self/status: gone"), "{error}");
}
