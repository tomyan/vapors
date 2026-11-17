//! GPU tier: the batch counts F3.1 is compared against
//! (`docs/perf/f3-baselines-2026-11-17.md`), pinned: the last frame's
//! draw calls, batches written and batches kept on the two off-path
//! pages that stand for W1 and W4, with each of their changes. The doc's
//! command counts frame 24 of a sampled run (5 warmup, 20 timed), so
//! after 25 changes; these tests reach the same frame counted alone
//! (`counted_frames.rs`): 24 changes with no frame between them, a
//! priming frame, the 25th change and the measured frame. A change to
//! any count fails here, and the doc's table changes with it.

use vapors_gpu_harness::gpu_with;
use vapors_perf::counted_frames::last_frame_counts;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::off_path::{Kind, OffPath, OffPathChange};

/// The doc's smallest page; its counts are the same at 3,000 and
/// 10,000.
const UNITS: usize = 1000;
/// The changes before the frame the doc's command counts: the scroll
/// changes' counts depend on how far they have scrolled, so other
/// numbers give other counts.
const CHANGES: usize = 25;

/// The last frame's (draw calls, batches written, batches kept) after
/// `CHANGES` of `change`, on its own GPU.
fn last_frame(kind: Kind, change: OffPathChange) -> (u64, u64, u64) {
    let gpu = gpu_with(FEATURES);
    let mut workload = OffPath::new(kind, UNITS, change);
    let frame = last_frame_counts(&gpu, &mut workload, CHANGES)
        .unwrap_or_else(|error| panic!("{kind:?} {change:?}: {error}"));
    (
        frame.total().draw_calls,
        frame.batches_written,
        frame.batches_kept,
    )
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
