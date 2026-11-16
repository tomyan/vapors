//! The kept steps patched when a splice only moves framed lane rows
//! (4f-2b): rows enter and leave at both edges of the view, and the
//! steps found are a full search's (the debug check after every draw,
//! `assert_candidates`) with no full search made.

use vapors_dom::{Dom, NodeId};
use vapors_scene::Rect;

use crate::document::Document;
use crate::update::Viewport;

/// 300 x 210 device pixels: ten and a half 20 px rows.
const VIEWPORT: Viewport = Viewport {
    width: 300.0,
    height: 210.0,
    scale: 1.0,
};

/// A list of 200 rows, framed, its top row's lower half at the view's
/// top and its eleventh row cut by the view's bottom; drawn.
fn scrolled_list() -> (Document, NodeId) {
    let mut doc = Document::new(vapors_text::FontSystem::bundle_only());
    let css = "html { overflow: hidden; } body { margin: 0; overflow: auto; height: 210px; } \
        .row { height: 20px; background-color: #f00; }";
    doc.add_stylesheet("p.css", css).expect("parses");
    let list = doc.create_element("div");
    doc.append(doc.body(), list);
    for _ in 0..200 {
        put_row(&mut doc, list, None);
    }
    doc.update(VIEWPORT).expect("updates");
    put_row(&mut doc, list, None);
    doc.update(VIEWPORT).expect("updates");
    assert!(
        doc.scroll_node_by(doc.body(), [0.0, 1010.0]),
        "the body scrolls"
    );
    drawn(&mut doc);
    (doc, list)
}

/// A row put in `list` before `before` (at the end for `None`).
fn put_row(doc: &mut Document, list: NodeId, before: Option<NodeId>) -> NodeId {
    let row = doc.create_element("div");
    doc.set_attribute(row, "class", "row");
    doc.insert_before(list, row, before);
    row
}

/// `doc` updated and its batches drawn; how many full searches the draw
/// made.
fn drawn(doc: &mut Document) -> usize {
    let builds = || crate::kept_batches::BUILDS.with(std::cell::Cell::get);
    let before = builds();
    let painted = doc.update(VIEWPORT).expect("updates");
    _ = painted.kept_batches(Rect::new(0.0, 0.0, 300.0, 210.0));
    builds() - before
}

#[test]
fn rows_put_in_and_taken_out_around_the_view_patch_the_steps_found() {
    // Given
    let (mut doc, list) = scrolled_list();
    let row = |doc: &Document, at: usize| doc.children(list)[at];

    // When / Then: above the view (every row in view moves down a row,
    // one enters at the top edge and one leaves at the bottom)
    let first = row(&doc, 0);
    put_row(&mut doc, list, Some(first));
    assert_eq!(drawn(&mut doc), 0, "patched above");
    assert_eq!(
        doc.update_counts().batch_steps_sorted_again,
        0,
        "counted as patched"
    );
    // taken out above (rows move up: one enters at the bottom)
    let second = row(&doc, 1);
    doc.remove(second);
    assert_eq!(drawn(&mut doc), 0, "patched taken out");
    // inside the view, and at the bottom edge
    let inside = row(&doc, 53);
    put_row(&mut doc, list, Some(inside));
    assert_eq!(drawn(&mut doc), 0, "patched inside");
    let bottom = row(&doc, 60);
    put_row(&mut doc, list, Some(bottom));
    assert_eq!(drawn(&mut doc), 0, "patched at the bottom edge");
}

#[test]
fn rows_put_in_at_two_places_in_view_until_a_gap_relabels_patch_the_steps_found() {
    // Given
    let (mut doc, list) = scrolled_list();
    let (first, second) = (doc.children(list)[53], doc.children(list)[57]);

    // When: rows put in before two rows in view in turn, each drawn,
    // until a gap between labels runs out and the splice relabels
    let mut relabelled = 0;
    for turn in 0..150 {
        put_row(
            &mut doc,
            list,
            Some(if turn % 2 == 0 { first } else { second }),
        );
        drawn(&mut doc);
        relabelled += doc.update_counts().paint_relabelled;
    }

    // Then: some splice relabelled, and every draw's steps were a full
    // search's (the debug check in `KeptBatches::for_view`)
    assert!(
        relabelled > 0,
        "no gap ran out in 150 inserts: the case was not reached"
    );
}

#[test]
fn a_splice_that_relabels_a_shown_step_moves_it_to_its_new_label() {
    // Given: the steps shown in the scrolled list
    let (doc, _) = scrolled_list();
    let painted = doc.painted.as_ref().expect("painted");
    let view = Rect::new(0.0, 0.0, 300.0, 210.0);
    let mut kept = crate::kept_batches::KeptBatches::of(view, painted.batch_sources());
    let (first, second) = (kept.shown[0], kept.shown[1]);

    // When: a splice relabels the first into the gap before the second
    let new = first.label + (second.label - first.label) / 2;
    let edits = crate::kept_order_splice::SpliceEdits {
        relabelled: vec![(first.step, first.label, new)],
        ..Default::default()
    };
    kept.spliced(&edits);

    // Then
    assert_eq!(kept.shown[0].label, new);
    assert_eq!(kept.shown[0].step, first.step);
}

#[test]
fn a_box_taken_out_of_a_small_page_is_taken_out_of_the_steps_shown() {
    // Given: three boxes, drawn
    let mut doc = Document::new(vapors_text::FontSystem::bundle_only());
    doc.add_stylesheet("p.css", "div { height: 20px; background-color: #f00; }")
        .expect("parses");
    let boxes: Vec<NodeId> = (0..3)
        .map(|_| {
            let node = doc.create_element("div");
            doc.append(doc.body(), node);
            node
        })
        .collect();
    drawn(&mut doc);

    // When: the middle one taken out
    doc.remove(boxes[1]);
    drawn(&mut doc);

    // Then: the steps shown and the items counted are a full search's
    // (the debug checks in `KeptBatches::for_view` and `culled`)
}

#[test]
fn the_first_row_in_view_taken_out_sorts_the_steps_again() {
    // Given
    let (mut doc, list) = scrolled_list();

    // When: the row the view's top cuts taken out, an end row the lane no
    // longer holds
    let top = doc.children(list)[50];
    doc.remove(top);
    doc.update(VIEWPORT).expect("updates");
    let counted = doc.update_counts().batch_steps_sorted_again;
    let builds = drawn(&mut doc);

    // Then: the update counted the fallback, and the steps were sorted
    // again (and are a full search's: the debug check in
    // `KeptBatches::for_view`)
    assert_eq!((counted, builds), (1, 1));
}

/// The lane batches' tables and each one's items' slots in the batches
/// drawn of `doc`.
fn lane_tables(doc: &mut Document) -> Vec<(Vec<usize>, Vec<u32>)> {
    let painted = doc.update(VIEWPORT).expect("updates");
    let kept = painted.kept_batches(Rect::new(0.0, 0.0, 300.0, 210.0));
    let lane = kept.rows.iter().filter(|rows| !rows.items.is_empty());
    lane.map(|rows| (rows.frames.to_vec(), rows.items.to_vec()))
        .collect()
}

#[test]
fn the_lane_batches_of_a_part_share_one_table() {
    // Given
    let (mut doc, _) = scrolled_list();

    // When
    let tables = lane_tables(&mut doc);

    // Then: one table for every lane batch, so they draw through one
    // uniform slot
    assert_ne!(tables.len(), 0);
    assert!(
        tables.iter().all(|(table, _)| *table == tables[0].0),
        "{tables:?}"
    );
}

#[test]
fn a_row_put_in_above_takes_a_free_slot_and_no_other_row_moves_slot() {
    // Given: the rows in view and their slots
    let (mut doc, list) = scrolled_list();
    let before = lane_tables(&mut doc).remove(0).0;

    // When: a row put in at the top: one row leaves the view at the
    // bottom, one enters at the top
    let first = doc.children(list)[0];
    put_row(&mut doc, list, Some(first));
    let after = lane_tables(&mut doc).remove(0).0;

    // Then: every row in view both times holds the slot it held
    let slot_of = |table: &[usize], row: usize| table.iter().position(|&held| held == row);
    let stayed = before.iter().filter(|&&row| after.contains(&row));
    for &row in stayed {
        assert_eq!(slot_of(&after, row), slot_of(&before, row), "row {row}");
    }
}
