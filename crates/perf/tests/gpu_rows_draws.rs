//! GPU tier: the draw calls of the rows page's idle frame, pinned. Each
//! row paints its label's text, its value's text under the value's clip
//! (drawn for the 17 rows in view only), its button's quad and its
//! button's text; consecutive texts under one clip are one run and one
//! draw.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

const FEW: Settings = Settings { warmup: 1, runs: 1 };

#[test]
fn gpu_the_rows_pages_idle_frame_draws_one_text_run_per_run_not_per_text() {
    // Given
    let mut rows = Rows::new(100, RowChange::Idle);
    let gpu = gpu_with(FEATURES);

    // When
    let samples = Sampler::new(&gpu)
        .sample(&mut rows, FEW)
        .expect("the frames ran");

    // Then: 101 unclipped runs (the first label; each button's text with
    // the next row's label; the last button's text), 17 values in view
    let frame = samples.counts.frame;
    assert_eq!(frame.texts.draw_calls, 118, "{frame:?}");
    assert_eq!(frame.quads.draw_calls, 100, "{frame:?}");
    assert_eq!(frame.total().draw_calls, 218, "{frame:?}");
}
