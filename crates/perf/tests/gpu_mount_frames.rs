//! GPU tier: the Mount page's first frame as a user waits for it
//! (`Sampler::first_frame`): `App::start` holds the first update, then
//! frame 0 draws, timed to submit and to the GPU's idle.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::instructions_check::checked_counter;
use vapors_perf::mount::Mount;
use vapors_perf::run::Workload;
use vapors_perf::sampler::Sampler;
use vapors_perf::spans::PhaseClock;

const ROWS: usize = 300;

#[test]
fn gpu_the_first_update_is_in_the_start_and_frame_0_is_timed_to_idle() {
    // Given
    let mut mount = Mount::new(ROWS, checked_counter().expect("perf events"));
    let gpu = gpu_with(FEATURES);
    let clock = PhaseClock::with_instructions(checked_counter().expect("perf events"));

    // When
    let first = Sampler::with_clock(&gpu, clock)
        .first_frame(&mut mount)
        .expect("started and drawn");

    // Then: the rows were styled in the start, and frame 0 restyled none
    let style = first
        .start_phases
        .1
        .as_ref()
        .and_then(|counts| counts.get("style"));
    assert!(
        style.is_some_and(|&count| count > 0),
        "{:?}",
        first.start_phases
    );
    assert_eq!(mount.counts().update.elements_restyled, 0);
    assert_eq!(mount.counts().update.styles_live, 6);
    assert!(first.frame_idle >= first.frame_cpu);
    assert!(first.frame_instructions.is_some());
    assert_eq!(mount.mounted.len(), 1);
}

#[test]
fn gpu_a_failed_mount_ends_the_first_frame_with_its_cause_not_a_panic() {
    // Given
    let mut mount = Mount::new(10, checked_counter().expect("perf events"));
    mount.fail_for_test("reading /proc/self/status: gone");
    let gpu = gpu_with(FEATURES);

    // When
    let result = Sampler::new(&gpu).first_frame(&mut mount);

    // Then
    let error = result.expect_err("the mount failed");
    assert!(error.contains("reading /proc/self/status: gone"), "{error}");
}
