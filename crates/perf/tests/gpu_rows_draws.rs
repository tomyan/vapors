//! GPU tier: the draw calls of the rows page's idle frame, pinned. Each
//! row paints its label's text, its value's text under the value's
//! clip, its button's quad and its button's text; consecutive texts are
//! one run and one draw, each naming its clip (flat-scene F3.1b). Only the rows in view are
//! drawn: the rest are culled, so the draws do not grow with the rows.

use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::rows::{RowChange, Rows, rows_in_view};
use vapors_perf::run::Settings;
use vapors_perf::sampler::Sampler;

/// Many more rows than the 17 in view.
const ROWS: usize = 300;
/// Items each row paints: label text, value text, button quad, button text.
const ITEMS_PER_ROW: usize = 4;
const FEW: Settings = Settings { warmup: 1, runs: 1 };

#[test]
fn gpu_the_rows_pages_idle_frame_draws_the_rows_in_view_only() {
    // Given
    let mut rows = Rows::new(ROWS, RowChange::Idle);
    let gpu = gpu_with(FEATURES);

    // When
    let samples = Sampler::new(&gpu)
        .sample(&mut rows, FEW)
        .expect("the frames ran");

    // Then
    let frame = samples.counts.frame;
    let in_view = rows_in_view();
    // Text runs, the value's text joined with the texts either side of
    // it under its own clip (flat-scene F3.1b; one run per value more
    // before, 35): the first row's label and value, each button's text
    // with the next row's label and value (in view - 1), the last
    // button's text in view. Each row is a lane frame of its own; a batch
    // spans them, each item at its row's offset (4f). The buttons' quads
    // between them keep the runs apart.
    let texts = 1 + (in_view - 1) + 1;
    assert_eq!(frame.texts.draw_calls, texts as u64, "{frame:?}");
    // One quad draw per row in view: its button.
    assert_eq!(frame.quads.draw_calls, in_view as u64, "{frame:?}");
    assert_eq!(
        frame.total().draw_calls,
        (texts + in_view) as u64,
        "{frame:?}"
    );
    let culled = (ROWS - in_view) * ITEMS_PER_ROW;
    assert_eq!(frame.culled, culled as u64, "{frame:?}");
}
