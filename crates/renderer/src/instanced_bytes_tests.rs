//! The instanced pipelines' row indexes (`instanced_bytes.rs`
//! `assert_rows`): one an item, each below the most rows a batch's range
//! holds (`ROWS_PER_BATCH`, 128), written beside the frame's entry.

use vapors_scene::{Color, Quad, Rect, Shadow};

use crate::{QuadPipeline, ShadowPipeline};

/// Two quads, for the row indexes given with them.
fn two_quads() -> [Quad; 2] {
    let quad = Quad::fill(
        Rect::new(0.0, 0.0, 1.0, 1.0),
        Color::from_rgba8(1, 2, 3, 255),
    );
    [quad; 2]
}

#[test]
fn an_item_in_the_last_row_a_batch_holds_is_written_with_its_frame() {
    // When: row 127, the last of 128
    let bytes = QuadPipeline::bytes_in(&two_quads(), (9, &[]), &[0, 127]);

    // Then: the second quad's row, then the frame's entry after it
    assert_eq!(bytes[144 + 128..144 + 132], 127u32.to_le_bytes());
    assert_eq!(bytes[144 + 132..144 + 136], 9u32.to_le_bytes());
}

#[test]
#[should_panic(
    expected = "quads: item 1 names row 128, past the 128 rows a batch's range holds; split the run into batches of at most that many rows (vapors-scene batch.rs)"
)]
fn an_item_past_the_rows_a_batch_holds_is_refused_naming_the_item_and_the_row() {
    let _ = QuadPipeline::bytes_in(&two_quads(), (0, &[]), &[0, 128]);
}

#[test]
#[should_panic(expected = "shadows: 1 row indexes for 2 items")]
fn fewer_rows_than_items_are_refused() {
    let shadow = Shadow {
        rect: Rect::new(0.0, 0.0, 2.0, 2.0),
        radii: [0.0; 4],
        blur: 1.0,
        color: Color::from_rgba8(0, 0, 0, 255),
        caster: Rect::new(0.0, 0.0, 2.0, 2.0),
        caster_radii: [0.0; 4],
    };
    let _ = ShadowPipeline::bytes_in(&[shadow, shadow], (0, &[]), &[0]);
}

#[test]
fn each_item_names_the_clip_entry_of_its_run() {
    // Given: three quads, the first unclipped, the last two under entry 7
    let quads = [two_quads()[0]; 3];

    // When
    let bytes = QuadPipeline::bytes_in(&quads, (9, &[(0, 0), (1, 7)]), &[]);

    // Then: each instance's clip index, after its row and frame
    let clip = |item: usize| bytes[item * 144 + 136..item * 144 + 140].to_vec();
    assert_eq!(clip(0), 0u32.to_le_bytes());
    assert_eq!(clip(1), 7u32.to_le_bytes());
    assert_eq!(clip(2), 7u32.to_le_bytes());
}

#[test]
#[should_panic(expected = "quads: clip runs [(0, 7), (1, 8), (1, 9)] for 2 items")]
fn clip_runs_out_of_order_are_refused() {
    let _ = QuadPipeline::bytes_in(&two_quads(), (0, &[(0, 7), (1, 8), (1, 9)]), &[]);
}

#[test]
#[should_panic(
    expected = "quads: clip runs [(1, 7)] for 2 items: the first run must start at item 0"
)]
fn clip_runs_that_leave_the_first_item_out_are_refused() {
    let _ = QuadPipeline::bytes_in(&two_quads(), (0, &[(1, 7)]), &[]);
}
