use vapors_dom::Dom;
use vapors_elements::{BatchRows, Document, RowsOfBatches, Viewport};
use vapors_scene::Rect;
use vapors_text::FontSystem;

use super::*;

/// A key the page made for its one batch.
fn a_key() -> BatchKey {
    one_batch().0
}

/// The frame of the page's one batch.
fn a_frame() -> vapors_elements::BatchFrame {
    one_batch().1
}

/// The key and the frame of the one batch of a page of one red div.
fn one_batch() -> (BatchKey, vapors_elements::BatchFrame) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", "div { height: 2px; background-color: #f00; }")
        .expect("parses");
    let div = doc.create_element("div");
    doc.append(doc.body(), div);
    let view = Viewport {
        width: 10.0,
        height: 10.0,
        scale: 1.0,
    };
    let painted = doc.update(view).expect("updates");
    let kept = painted.kept_batches(Rect::new(0.0, 0.0, 10.0, 10.0));
    (kept.keys[0], kept.frames[0])
}

#[test]
fn a_batch_drawn_at_the_same_offsets_keeps_its_table() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    let kept = tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    let again = tables.table(key, &[[0.0, 4.0]]);

    // Then
    assert_eq!(again, kept);
    assert_eq!(again, RowOffsets::whole(&[[0.0, 4.0]]));
}

#[test]
fn a_batch_drawn_at_other_offsets_gets_a_table_made_from_them() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    let moved = tables.table(key, &[[0.0, 8.0]]);

    // Then
    assert_eq!(moved, RowOffsets::whole(&[[0.0, 8.0]]));
}

#[test]
fn a_batch_not_drawn_in_a_frame_has_its_table_dropped() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    tables.release_undrawn();

    // Then
    assert!(tables.tables.is_empty());
}

#[test]
fn a_batch_drawn_again_keeps_its_table_past_the_frames_end() {
    // Given
    let (mut tables, key) = (RowTables::default(), a_key());
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);

    // When
    tables.begin();
    tables.table(key, &[[0.0, 4.0]]);
    tables.release_undrawn();

    // Then
    assert_eq!(tables.tables.len(), 1);
}

/// The rows of the batches on a page of 40 coloured rows.
fn page_rows() -> RowsOfBatches {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } div { height: 2px; background-color: #f00; }",
    )
    .expect("parses");
    for _ in 0..40 {
        let div = doc.create_element("div");
        doc.append(doc.body(), div);
    }
    let view = Viewport {
        width: 10.0,
        height: 10.0,
        scale: 1.0,
    };
    let painted = doc.update(view).expect("updates");
    painted
        .kept_batches(Rect::new(0.0, 0.0, 10.0, 10.0))
        .rows
        .clone()
}

/// The first batch over a lane's rows among `rows`.
fn lane_rows(rows: &RowsOfBatches) -> BatchRows<'_> {
    let lane = rows.iter().find(|rows| !rows.items().is_empty());
    lane.expect("a batch over the rows")
}

/// A batch at a shift of 3, 5 with `rows` at `offsets`.
fn moved<'a>(rows: BatchRows<'a>, offsets: &'a [[f32; 2]]) -> Moved<'a> {
    Moved {
        frame: a_frame(),
        shift: Shift::whole([3.0, 5.0]),
        table: &RowOffsets::NONE,
        offsets,
        rows,
    }
}

#[test]
fn an_item_of_a_batch_in_one_frame_is_drawn_at_the_batchs_shift() {
    // Given
    let rows = BatchRows::default();

    // When / Then
    assert_eq!(moved(rows, &[]).item_shift(0), Shift::whole([3.0, 5.0]));
}

#[test]
fn an_item_of_a_lane_batch_is_drawn_at_the_shift_and_its_rows_offset() {
    // Given
    let page = page_rows();
    let rows = lane_rows(&page);
    let offsets = vec![[0.0, 2.0]; rows.items().len()];

    // When / Then
    assert_eq!(
        moved(rows, &offsets).item_shift(0),
        Shift::whole([3.0, 7.0])
    );
}

#[test]
#[should_panic(expected = "item 0 of a lane batch has no row with an offset")]
fn an_item_of_a_lane_batch_whose_row_has_no_offset_is_refused() {
    let page = page_rows();
    let rows = lane_rows(&page);
    moved(rows, &[]).item_shift(0);
}

#[test]
#[should_panic(expected = "has no row with an offset")]
fn an_item_past_a_lane_batchs_rows_is_refused() {
    let page = page_rows();
    let rows = lane_rows(&page);
    let offsets = vec![[0.0, 2.0]; rows.items().len()];
    moved(rows, &offsets).item_shift(rows.items().len());
}
