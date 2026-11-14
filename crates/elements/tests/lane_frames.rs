//! A lane's children are frames of their own (`docs/plans/splice-
//! layout.md`, section 11): after an insert in a list no later row moves
//! or is painted, shifted or placed for it; and a lane whose children
//! cannot be frames (a fractional start, a scroll container in a row) is
//! counted and keeps the placement from before. Each case also matches a
//! full update, and the debug checks compare placement, frames and paint
//! with ones made afresh.

mod support;

use support::mirror_model::Model;
use support::narrow_list::{List, ROWS, VIEW, list};
use support::oracle::assert_matches_full;
use vapors_dom::Dom;
use vapors_elements::{LaneDecline, UpdateCounts};

/// The counts of the update after `edit` on a list of `rows`, styled
/// `css` besides the rows, which matches a full update.
fn after(rows: usize, css: &str, edit: impl Fn(&List)) -> UpdateCounts {
    let list = list(rows, &format!("{ROWS} {css}"));
    edit(&list);
    let counts = list.update();
    assert_matches_full(&list.doc, VIEW(), "after the edit");
    counts
}

#[test]
fn an_insert_moves_paints_and_shifts_no_later_row() {
    // When: a row inserted in the middle of 100 rows and of 500
    let small = after(100, "", |list| _ = list.insert_at(50));
    let large = after(500, "", |list| _ = list.insert_at(250));

    // Then: the rows after it keep their rects, each in its own frame;
    // only the list and body, which grow, move, at either size
    assert_eq!(
        (small.boxes_moved, small.boxes_shifted),
        (2, 0),
        "{small:?}"
    );
    assert_eq!(small.lanes_unframed, 0, "{small:?}");
    assert_eq!(
        (small.boxes_moved, small.boxes_painted),
        (large.boxes_moved, large.boxes_painted),
        "placement or painting grows with the rows"
    );
}

#[test]
fn a_remove_moves_no_later_row() {
    let counts = after(100, "", |list| list.doc.borrow_mut().remove(list.rows[50]));
    // the list and body, which shrink, alone
    assert_eq!(
        (counts.boxes_moved, counts.boxes_shifted),
        (2, 0),
        "{counts:?}"
    );
}

#[test]
fn a_list_with_a_fractional_row_is_counted_and_placed_as_before() {
    // Given: one row a fraction of a pixel taller, so later starts are
    // not whole device pixels
    let css = ".odd { height: 20.5px; }";

    // When
    let counts = after(100, css, |list| {
        list.doc
            .borrow_mut()
            .set_attribute(list.rows[10], "class", "row odd");
    });

    // Then: the list is not framed; its later rows move as before
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFramePixel),
        "{counts:?}"
    );
    assert!(counts.boxes_moved > 0, "{counts:?}");
}

#[test]
fn a_list_with_a_scroller_in_a_row_is_counted_and_placed_as_before() {
    // Given: one row's cell scrolls
    let css = ".scroll { overflow: auto; }";

    // When
    let counts = after(100, css, |list| {
        let mut doc = list.doc.borrow_mut();
        let cell = doc.children(list.rows[10])[0];
        doc.set_attribute(cell, "class", "cell scroll");
    });

    // Then
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFrameScroller),
        "{counts:?}"
    );
}

#[test]
fn a_short_list_is_not_framed_and_not_counted() {
    // When: fewer rows than a lane frames
    let counts = after(10, "", |list| _ = list.insert_at(5));

    // Then: placed as before, and no decline
    assert!(counts.boxes_moved > 0, "{counts:?}");
    assert_eq!(counts.lanes_unframed, 0, "{counts:?}");
}

/// Draws `list` at its view and asserts the draw gets the batches of
/// the whole scene in its frames, naming `case`.
fn assert_drawn_afresh(list: &List, case: &str) {
    let mut doc = list.doc.borrow_mut();
    let painted = doc.update(VIEW()).expect("updates");
    let view = vapors_scene::Rect::new(0.0, 0.0, 400.0, 300.0);
    let kept = painted.kept_batches(view).clone();
    let fresh = painted.scene().framed_batches(view);
    assert_eq!((kept.batched, kept.frames), fresh, "{case}");
}

#[test]
fn a_row_resized_in_view_draws_the_rows_after_it_where_they_start_now() {
    // Given: a framed list drawn once, its first rows in view, the
    // second row tall
    let list = list(100, &format!("{ROWS} .tall {{ height: 60px; }}"));
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[1], "class", "row tall");
    assert_drawn_afresh(&list, "first draw");

    // When: the second row shrinks, so every later row starts higher
    // and rows below the view come into it, and no paint order changes
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[1], "class", "row");

    // Then: the draw finds the rows now in view, not those of before
    assert_drawn_afresh(&list, "after the resize");
}

#[test]
fn a_row_resized_in_a_framed_list_keeps_the_a11y_mirror_right() {
    // Given: a framed list of list items and a mirror built from its
    // first take
    let list = list(100, &format!("{ROWS} .tall {{ height: 60px; }}"));
    let first = {
        let mut doc = list.doc.borrow_mut();
        for &row in &list.rows {
            doc.set_attribute(row, "role", "listitem");
        }
        doc.update(VIEW()).expect("updates");
        doc.take_ax_update().expect("updated").expect("every node")
    };
    let mut model = Model::from_every(&first);

    // When: the second row grows, so every later row starts lower while
    // none moves in its frame
    list.doc
        .borrow_mut()
        .set_attribute(list.rows[1], "class", "row tall");
    let update = {
        let mut doc = list.doc.borrow_mut();
        doc.update(VIEW()).expect("updates");
        doc.take_ax_update().expect("updated")
    };

    // Then: the take holds the later rows' new bounds
    if let Some(update) = &update {
        model.apply(update);
    }
    model.assert_mirrors(&list.doc.borrow());
}
