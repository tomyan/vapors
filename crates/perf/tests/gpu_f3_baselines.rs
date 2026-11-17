//! GPU tier: the batch counts F3.1 is compared against
//! (`docs/perf/f3-baselines-2026-11-17.md`), pinned: the last frame's
//! draw calls, batches written and batches kept on the two off-path
//! pages that stand for W1 and W4, with each of their changes, run as
//! the doc's command runs them. A change to any count fails here, and
//! the doc's table changes with it.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::off_path::{Kind, OffPath, OffPathChange};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

/// The doc's smallest page; its counts are the same at 3,000 and
/// 10,000.
const UNITS: usize = 1000;
/// The doc's command's, `vapors-perf 20 5`: the scroll changes' counts
/// depend on how far they have scrolled, so fewer frames give others.
const AS_MEASURED: Settings = Settings {
    warmup: 5,
    runs: 20,
};

/// The last frame's (draw calls, batches written, batches kept) with
/// `change`, on its own GPU, one test each so each stays inside the
/// tier's deadline.
fn last_frame(kind: Kind, change: OffPathChange) -> (u64, u64, u64) {
    let gpu = gpu_with(FEATURES);
    let mut workload = OffPath::new(kind, UNITS, change);
    let samples = Sampler::new(&gpu).sample(&mut workload, AS_MEASURED);
    let frame = samples
        .unwrap_or_else(|error| panic!("{kind:?} {change:?}: {error}"))
        .counts
        .frame;
    let draws = frame.total().draw_calls;
    (draws, frame.batches_written, frame.batches_kept)
}

#[test]
fn gpu_nested_scrollers_edit_draws_35_writing_0_keeping_35() {
    assert_eq!(
        last_frame(Kind::NestedScrollers, OffPathChange::Edit),
        (35, 0, 35)
    );
}

#[test]
fn gpu_nested_scrollers_scroll_draws_35_writing_2_keeping_33() {
    assert_eq!(
        last_frame(Kind::NestedScrollers, OffPathChange::Scroll),
        (35, 2, 33)
    );
}

#[test]
fn gpu_nested_scrollers_scroll_inner_draws_35_writing_1_keeping_34() {
    assert_eq!(
        last_frame(Kind::NestedScrollers, OffPathChange::ScrollInner),
        (35, 1, 34)
    );
}

#[test]
fn gpu_nested_scrollers_insert_draws_35_writing_0_keeping_35() {
    assert_eq!(
        last_frame(Kind::NestedScrollers, OffPathChange::Insert),
        (35, 0, 35)
    );
}

#[test]
fn gpu_code_blocks_edit_draws_14_writing_0_keeping_14() {
    assert_eq!(
        last_frame(Kind::CodeBlocks, OffPathChange::Edit),
        (14, 0, 14)
    );
}

#[test]
fn gpu_code_blocks_scroll_draws_17_writing_3_keeping_14() {
    assert_eq!(
        last_frame(Kind::CodeBlocks, OffPathChange::Scroll),
        (17, 3, 14)
    );
}

#[test]
fn gpu_code_blocks_scroll_inner_draws_14_writing_1_keeping_13() {
    assert_eq!(
        last_frame(Kind::CodeBlocks, OffPathChange::ScrollInner),
        (14, 1, 13)
    );
}

#[test]
fn gpu_code_blocks_insert_draws_14_writing_0_keeping_14() {
    assert_eq!(
        last_frame(Kind::CodeBlocks, OffPathChange::Insert),
        (14, 0, 14)
    );
}
