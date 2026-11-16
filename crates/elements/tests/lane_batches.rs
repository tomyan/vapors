//! Batches across the rows of a framed lane (`docs/plans/splice-layout.md`
//! section 11.2, 4f): consecutive rows' items share a batch, each item
//! naming its row; a run over more rows than a batch's table holds
//! (`vapors_scene::ROWS_PER_BATCH`) is split there; and at a fractional
//! scale rows whole in device pixels are framed and batched, rows that
//! are not are declined, and both draw as the scene batched on the page
//! (the debug check in `kept_batches`).

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::lane_page::assert_drawn_afresh;
use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};

/// A list of `count` rows under body, each `height` CSS px with a
/// background, updated at `view`.
fn coloured_rows(count: usize, height: &str, view: Viewport) -> Rc<RefCell<Document>> {
    let css = format!("body {{ margin: 0; }} .row {{ height: {height}; background-color: #c33; }}");
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..count {
        element(&mut doc, list, "div", "row");
    }
    doc.add_stylesheet("page.css", &css).expect("parses");
    doc.update(view).expect("updates");
    Rc::new(RefCell::new(doc))
}

/// The rows of each batch drawn at `view`, those over a lane's rows.
fn batch_rows(doc: &Rc<RefCell<Document>>, view: Viewport) -> Vec<usize> {
    let mut doc = doc.borrow_mut();
    let painted = doc.update(view).expect("updates");
    let rect = vapors_scene::Rect::new(0.0, 0.0, view.width * view.scale, view.height * view.scale);
    let kept = painted.kept_batches(rect).clone();
    let rows = kept.rows.iter().map(|rows| {
        let items = rows.items();
        items.iter().max().map_or(0, |&most| most as usize + 1)
    });
    rows.filter(|&count| count > 0).collect()
}

#[test]
fn the_backgrounds_of_the_rows_in_view_are_one_batch() {
    // Given: 100 rows of 20 px, 15 in a 300 px view
    let view = viewport(400.0, 300.0);
    let doc = coloured_rows(100, "20px", view);

    // When
    let rows = batch_rows(&doc, view);

    // Then: one batch over the 15 rows' backgrounds, and the 16th's,
    // which starts at the view's bottom edge (culling keeps an item that
    // touches the view)
    assert_eq!(rows, [16]);
    assert_drawn_afresh(&doc, view, "one batch over the rows");
}

#[test]
fn a_run_over_more_rows_than_a_batch_holds_is_split_at_the_129th() {
    // Given: 1 px rows, 300 of them in the view
    let view = viewport(400.0, 300.0);
    let doc = coloured_rows(400, "1px", view);

    // When
    let rows = batch_rows(&doc, view);

    // Then: 128 rows, 128 more, then the 45 left (the 301st touches the
    // view's bottom edge)
    assert_eq!(rows, [128, 128, 45]);
    assert_drawn_afresh(&doc, view, "a run split by the table's capacity");
}

/// The view at the Pixel's scale, 2.75.
fn pixel_view() -> Viewport {
    Viewport {
        scale: 2.75,
        ..viewport(400.0, 300.0)
    }
}

#[test]
fn rows_whole_in_device_pixels_at_scale_2_75_are_batched_across_and_drawn_in_place() {
    // Given: rows of 4 px, 11 device px
    let doc = coloured_rows(200, "4px", pixel_view());

    // When
    let rows = batch_rows(&doc, pixel_view());

    // Then: framed, batched across, and drawn where the page has each
    assert!(rows.iter().all(|&count| count > 1), "{rows:?}");
    assert_eq!(doc.borrow().update_counts().lane_unframed, None);
    assert_drawn_afresh(&doc, pixel_view(), "whole rows at 2.75");
}

#[test]
fn rows_fractional_in_device_pixels_at_scale_2_75_are_declined_and_drawn_in_place() {
    // Given: rows of 20.5 px, 56.375 device px
    let doc = coloured_rows(200, "20.5px", pixel_view());

    // When
    let rows = batch_rows(&doc, pixel_view());

    // Then: declined, so no batch is over rows, and drawn as the page has it
    assert!(rows.is_empty(), "{rows:?}");
    assert_drawn_afresh(&doc, pixel_view(), "fractional rows at 2.75");
}

/// The keys of the batches drawn at `view`.
fn batch_keys(doc: &Rc<RefCell<Document>>, view: Viewport) -> Vec<vapors_elements::BatchKey> {
    let mut doc = doc.borrow_mut();
    let painted = doc.update(view).expect("updates");
    let rect = vapors_scene::Rect::new(0.0, 0.0, view.width * view.scale, view.height * view.scale);
    painted.kept_batches(rect).keys.clone()
}

#[test]
fn a_row_put_in_above_takes_a_free_slot_and_the_other_rows_batches_keep_their_keys() {
    // Given: 1 px rows, 301 in view, in three parts' tables (128 slots
    // each), drawn
    let view = viewport(400.0, 300.0);
    let doc = coloured_rows(400, "1px", view);
    let keys = batch_keys(&doc, view);

    // When: a row put in at the top: every row moves down one, the last
    // leaves the view and gives back its slot in the third part
    {
        let mut doc = doc.borrow_mut();
        let list = doc.children(doc.body())[0];
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        let first = doc.children(list)[0];
        doc.insert_before(list, row, Some(first));
    }

    // Then: the new row, in that slot, is a batch of the third part's
    // table; the rows in the first two parts keep their slots, so their
    // batches draw the same bytes and keep their keys; the third part's
    // lost its last row; and the batches are those of batching afresh,
    // drawn where the page has each item
    let now = batch_keys(&doc, view);
    assert_eq!(now.len(), 4, "{:?}", batch_rows(&doc, view));
    assert_eq!(now[1..3], keys[0..2]);
    assert_drawn_afresh(&doc, view, "a row put in above three parts");
}

#[test]
fn rows_whose_items_return_later_in_a_run_split_within_the_table_and_draw_afresh() {
    // Given: 200 rows of 1 px, 300 in view, each with a `position:
    // relative` child that paints after the lane's rows, so each row's
    // items return in a later part of the run
    let view = viewport(400.0, 300.0);
    let css = "body { margin: 0; } .row { height: 1px; background-color: #c33; } \
        .lift { position: relative; width: 4px; height: 1px; background-color: #33c; }";
    let mut doc = document();
    let body = doc.body();
    let list = element(&mut doc, body, "div", "list");
    for _ in 0..200 {
        let row = element(&mut doc, list, "div", "row");
        element(&mut doc, row, "div", "lift");
    }
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.update(view).expect("updates");
    let doc = Rc::new(RefCell::new(doc));

    // When
    let rows = batch_rows(&doc, view);

    // Then: no batch numbers more rows than a table holds, and the
    // batches are those of batching afresh, drawn where the page has them
    assert!(
        rows.iter()
            .all(|&count| count <= vapors_scene::ROWS_PER_BATCH),
        "{rows:?}"
    );
    assert!(rows.len() > 1, "the case is not reached: {rows:?}");
    assert_drawn_afresh(&doc, view, "rows returning within a run");
}
