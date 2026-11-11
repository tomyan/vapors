//! GPU tier: the draw calls of the rows page's idle frame, pinned. Each
//! row paints its label's text, its value's text under the value's clip
//! (drawn for the 17 rows in view only), its button's quad and its
//! button's text; consecutive texts under one clip are one run and one
//! draw.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::rows::{RowChange, Rows, rows_in_view};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

const ROWS: usize = 100;
const FEW: Settings = Settings { warmup: 1, runs: 1 };

#[test]
fn gpu_the_rows_pages_idle_frame_draws_one_text_run_per_run_not_per_text() {
    // Given
    let mut rows = Rows::new(ROWS, RowChange::Idle);
    let gpu = gpu_with(FEATURES);

    // When
    let samples = Sampler::new(&gpu)
        .sample(&mut rows, FEW)
        .expect("the frames ran");

    // Then
    let frame = samples.counts.frame;
    // Unclipped runs: the first label, each button's text with the next
    // row's label (rows - 1), the last button's text.
    let unclipped_runs = 1 + (ROWS - 1) + 1;
    // One run per value in view; the clipped-away values draw nothing.
    let texts = unclipped_runs + rows_in_view();
    assert_eq!(frame.texts.draw_calls, texts as u64, "{frame:?}");
    // One quad draw per row: its button.
    assert_eq!(frame.quads.draw_calls, ROWS as u64, "{frame:?}");
    assert_eq!(frame.total().draw_calls, (texts + ROWS) as u64, "{frame:?}");
}
