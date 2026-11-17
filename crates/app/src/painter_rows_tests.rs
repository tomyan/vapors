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
        offsets,
        rows_key: a_key(),
        rows,
    }
}

#[test]
fn an_item_of_a_batch_in_one_frame_is_in_no_row() {
    // Given
    let rows = BatchRows::default();

    // When / Then
    assert_eq!(moved(rows, &[]).item_row(0), None);
}

#[test]
fn an_item_of_a_lane_batch_is_in_its_row() {
    // Given
    let page = page_rows();
    let rows = lane_rows(&page);
    let offsets = vec![[0.0, 2.0]; rows.items().len()];

    // When / Then
    assert_eq!(moved(rows, &offsets).item_row(0), Some(rows.items()[0]));
}

#[test]
#[should_panic(expected = "item 0 of a lane batch has no row with an offset")]
fn an_item_of_a_lane_batch_whose_row_has_no_offset_is_refused() {
    let page = page_rows();
    let rows = lane_rows(&page);
    moved(rows, &[]).item_row(0);
}

#[test]
#[should_panic(expected = "has no row with an offset")]
fn an_item_past_a_lane_batchs_rows_is_refused() {
    let page = page_rows();
    let rows = lane_rows(&page);
    let offsets = vec![[0.0, 2.0]; rows.items().len()];
    moved(rows, &offsets).item_row(rows.items().len());
}

#[test]
fn every_item_row_inside_its_batchs_range_is_taken() {
    super::super::kept::rows_in_range(a_key(), &[0, 2, 1], 3);
    super::super::kept::rows_in_range(a_key(), &[], 0);
}

#[test]
#[should_panic(expected = "names row 3, past the 3 rows of the batch's range")]
fn an_item_row_at_the_end_of_its_batchs_range_is_refused() {
    super::super::kept::rows_in_range(a_key(), &[0, 3], 3);
}

#[test]
#[should_panic(expected = "names row 0, past the 0 rows")]
fn an_item_row_in_a_batch_with_no_range_is_refused() {
    super::super::kept::rows_in_range(a_key(), &[0], 0);
}

#[test]
fn kept_bytes_need_their_highest_row_and_one() {
    assert_eq!(super::super::kept::rows_needed(&[0, 4, 2]), 5);
    assert_eq!(super::super::kept::rows_needed(&[]), 0);
}

#[test]
fn kept_bytes_fit_a_range_holding_the_rows_they_name() {
    super::super::kept::rows_fit(a_key(), 5, 5);
    super::super::kept::rows_fit(a_key(), 0, 0);
}

#[test]
#[should_panic(expected = "kept bytes name 5 rows, and its range holds 4")]
fn kept_bytes_drawn_with_a_range_that_shrank_are_refused() {
    super::super::kept::rows_fit(a_key(), 5, 4);
}
