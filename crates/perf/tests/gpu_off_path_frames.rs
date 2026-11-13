//! GPU tier: every off-path page runs its frames with each of its
//! changes, small, with no failure from the app.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::off_path::{Kind, OffPath};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

/// More units than the viewport shows, so scrolls move and units are culled.
const UNITS: usize = 60;
const FEW: Settings = Settings { warmup: 2, runs: 4 };

#[test]
fn gpu_every_off_path_page_runs_its_frames_with_each_of_its_changes() {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut timer = Sampler::new(&gpu);
    for kind in Kind::ALL {
        for &change in kind.changes() {
            let mut workload = OffPath::new(kind, UNITS, change);

            // When
            let samples = timer.sample(&mut workload, FEW);

            // Then
            let samples = samples.unwrap_or_else(|error| panic!("{kind:?} {change:?}: {error}"));
            assert_eq!(samples.frame.len(), FEW.runs, "{kind:?} {change:?}");
            assert!(samples.counts.frame.culled > 0, "{kind:?} {change:?}");
        }
    }
}
