//! Batches kept across draws (IU3 3c-3f, `docs/plans/iu3.md` "3c-3f:
//! kept batches"): after every kind of update and view change, a draw's
//! batches equal batching the whole scene afresh. A debug build also
//! makes that comparison inside every draw (`kept_batches_check.rs`).

mod support;

use support::element;
use support::rows_page::{EDITS, RowsPage, rows_page, rows_viewport};
use vapors_dom::Dom;
use vapors_elements::Viewport;
use vapors_scene::Rect;

/// The rows page's viewport in device pixels.
const VIEW: Rect = Rect {
    left: 0.0,
    top: 0.0,
    right: 640.0,
    bottom: 400.0,
};

/// Updates `page` at `viewport` and asserts that the draw at `view`
/// gets the batches of the whole scene, naming `case`.
fn assert_drawn_afresh(page: &RowsPage, viewport: Viewport, view: Rect, case: &str) {
    let mut doc = page.doc.borrow_mut();
    let painted = doc.update(viewport).expect("updates");
    let kept = painted.batches(view).clone();
    assert_eq!(kept, painted.scene().batches(view), "{case}");
}

#[test]
fn every_edit_in_and_out_of_view_draws_the_batches_of_the_whole_scene() {
    for rows in [10, 100] {
        // Given: drawn once; the middle row in view with 10 rows, below it with 100
        let page = rows_page(rows);
        assert_drawn_afresh(&page, rows_viewport(), VIEW, "first draw");

        for edit in EDITS {
            // When
            page.edit(edit);

            // Then
            let case = format!("{edit:?} on {rows} rows");
            assert_drawn_afresh(&page, rows_viewport(), VIEW, &case);
        }
    }
}

#[test]
fn a_draw_at_another_view_gets_that_views_batches() {
    // Given
    let page = rows_page(100);
    assert_drawn_afresh(&page, rows_viewport(), VIEW, "first draw");

    // When: the same paint drawn on a shorter target
    let short = Rect::new(0.0, 0.0, 640.0, 100.0);

    // Then
    assert_drawn_afresh(&page, rows_viewport(), short, "shorter view");
}

#[test]
fn a_row_put_in_after_a_draw_is_in_the_next_draws_batches() {
    // Given
    let page = rows_page(10);
    assert_drawn_afresh(&page, rows_viewport(), VIEW, "first draw");

    // When: paint order is walked again
    {
        let mut doc = page.doc.borrow_mut();
        let first = page.rows[0].element;
        let body = doc.body();
        let row = element(&mut doc, body, "div", "row marked");
        doc.insert_before(body, row, Some(first));
    }

    // Then
    assert_drawn_afresh(&page, rows_viewport(), VIEW, "row inserted");
}

#[test]
fn a_draw_after_the_scale_changes_gets_the_new_scales_batches() {
    // Given
    let page = rows_page(100);
    assert_drawn_afresh(&page, rows_viewport(), VIEW, "first draw");

    // When
    let doubled = Viewport {
        scale: 2.0,
        ..rows_viewport()
    };

    // Then: the same device view, now half the rows
    assert_drawn_afresh(&page, doubled, VIEW, "scale 2");
}
