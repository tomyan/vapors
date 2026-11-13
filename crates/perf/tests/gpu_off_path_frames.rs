//! GPU tier: every off-path page runs its frames with each of its
//! changes, small, with no failure from the app. One test per kind, so
//! each stays well inside the tier's deadline.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::off_path::{Kind, OffPath};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

/// More units than the viewport shows, so scrolls move and units are
/// culled: 200 grid cells are 29 rows of seven.
const UNITS: usize = 200;
const FEW: Settings = Settings { warmup: 2, runs: 4 };

fn runs_each_change(kind: Kind) {
    // Given
    let gpu = gpu_with(FEATURES);
    let mut timer = Sampler::new(&gpu);
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

#[test]
fn gpu_the_nested_scrollers_page_runs_its_frames() {
    runs_each_change(Kind::NestedScrollers);
}

#[test]
fn gpu_the_code_blocks_page_runs_its_frames() {
    runs_each_change(Kind::CodeBlocks);
}

#[test]
fn gpu_the_wrapped_cards_page_runs_its_frames() {
    runs_each_change(Kind::WrappedCards);
}

#[test]
fn gpu_the_grid_cells_page_runs_its_frames() {
    runs_each_change(Kind::GridCells);
}

#[test]
fn gpu_the_paragraphs_page_runs_its_frames() {
    runs_each_change(Kind::MixedText);
}

#[test]
fn every_kind_has_a_gpu_test_here() {
    assert_eq!(Kind::ALL.len(), 5, "add a gpu_ test above for the new kind");
}

#[test]
fn gpu_the_samples_record_the_warmup_the_sampler_ran() {
    // Given: an odd warmup; `of_kind`'s tests pin how it names the kinds
    let gpu = gpu_with(FEATURES);
    let mut workload = OffPath::new(
        Kind::CodeBlocks,
        UNITS,
        vapors_perf::off_path::OffPathChange::Insert,
    );
    let settings = Settings { warmup: 3, runs: 6 };

    // When
    let samples = Sampler::new(&gpu)
        .sample(&mut workload, settings)
        .expect("the frames ran");

    // Then
    assert_eq!(samples.warmup, 3);
}
