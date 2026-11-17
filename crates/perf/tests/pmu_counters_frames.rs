//! PMU tier (`cargo xtask test --tier pmu`, never run by ci: it needs
//! hardware counters and a GPU): with the hardware counters on the
//! phase clock (`--counters`), every kept frame has a reading, every
//! timed phase has counts, and the frame's instructions cover its
//! phases'.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::hardware_counters::{HardwareCounters, INSTRUCTIONS};
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;
use vapors_perf::spans::PhaseClock;

const SETTINGS: Settings = Settings { warmup: 2, runs: 5 };

#[test]
fn pmu_every_kept_frame_and_timed_phase_has_hardware_counts() {
    // Given
    let counters = HardwareCounters::open().expect("perf events for --counters");
    let gpu = gpu_with(FEATURES);
    let mut rows = Rows::new(100, RowChange::Text);

    // When
    let samples = Sampler::with_clock(&gpu, PhaseClock::with_counters(counters))
        .sample(&mut rows, SETTINGS)
        .expect("the frames ran");

    // Then
    let kept = samples.counters.expect("hardware counts");
    assert_eq!(kept.frame.len(), SETTINGS.runs);
    for phase in samples.phases.keys() {
        assert_eq!(kept.phases[phase].len(), SETTINGS.runs, "{phase}");
    }
    let update = &kept.phases["update"];
    for (frame, update) in kept.frame.iter().zip(update) {
        assert!(
            frame[INSTRUCTIONS] >= update[INSTRUCTIONS],
            "{frame:?} {update:?}"
        );
        assert!(update[INSTRUCTIONS] > 0, "{update:?}");
    }
    assert!(samples.instructions.is_none());
}
