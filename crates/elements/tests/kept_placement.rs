//! Placement kept across updates (IU3 3b, `docs/plans/iu3.md` "3b:
//! kept placement"): rects and scroll offsets are worked out again only
//! for the boxes Taffy laid out or wrote, the boxes built again, the
//! scrolled boxes, and the boxes placed in a box whose content moved.
//! A debug build also places every box afresh and compares
//! (`placement_check.rs`), so a box left out panics there too.

mod support;

use support::scroll_list::list;
use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, UpdateCounts};

/// A fixed-height page, so a growing row moves no ancestor, of rows
/// 100px wide, each one box and its text run.
const CSS: &str = "html, body { height: 400px; } body { margin: 0; } \
    .row { width: 100px; } \
    .holder { position: relative; } \
    .abs { position: absolute; top: 100px; left: 20px; width: 50px; }";

const ROWS: usize = 5;

/// `ROWS` rows of short text, updated once: the document and the rows'
/// text nodes.
fn rows() -> (Document, Vec<NodeId>) {
    let mut doc = document();
    doc.add_stylesheet("app.css", CSS).expect("parses");
    let body = doc.body();
    let texts = (0..ROWS)
        .map(|_| {
            let row = element(&mut doc, body, "div", "row");
            text(&mut doc, row, "a")
        })
        .collect();
    updated(&mut doc, 200.0);
    (doc, texts)
}

fn updated(doc: &mut Document, width: f32) -> UpdateCounts {
    doc.update(viewport(width, 400.0)).expect("updates");
    doc.update_counts()
}

#[test]
fn a_first_update_places_every_box() {
    // Given: a page never updated
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    text(&mut doc, row, "a");

    // When
    let counts = updated(&mut doc, 200.0);

    // Then: html, body, the row and its text run
    assert_eq!((counts.boxes_built, counts.boxes_placed), (4, 4));
}

#[test]
fn an_idle_update_places_nothing() {
    // Given
    let (mut doc, _) = rows();

    // When: an update with nothing changed
    let counts = updated(&mut doc, 200.0);

    // Then
    assert_eq!((counts.boxes_placed, counts.boxes_moved), (0, 0));
}

#[test]
fn a_row_whose_text_grows_moves_the_rows_after_it() {
    // Given
    let (mut doc, texts) = rows();

    // When: the second row's text wraps onto more lines
    doc.set_text(texts[1], "a much longer label that wraps in 100px");
    let counts = updated(&mut doc, 200.0);

    // Then: the three rows after it, each a box and a text run, moved;
    // the row itself was built again, and the rows before it stayed
    assert_eq!(counts.boxes_moved, 2 * (ROWS - 2));
}

#[test]
fn a_resize_places_every_box_again() {
    // Given
    let (mut doc, _) = rows();
    let boxes = 2 + 2 * ROWS;

    // When: the viewport narrows; every element is restyled on a
    // resize, so every box is built again
    let counts = updated(&mut doc, 150.0);

    // Then
    assert_eq!(counts.boxes_placed, boxes);
}

#[test]
fn a_scroll_moves_only_the_scrollers_descendants() {
    // Given: a scroll container of four rows, with a box after it
    let mut list = list("overflow: auto;");
    let body = list.doc.body();
    element(&mut list.doc, body, "div", "row");
    updated(&mut list.doc, 200.0);

    // When
    assert!(list.doc.scroll_node_by(list.container, [0.0, 30.0]));
    let counts = updated(&mut list.doc, 200.0);

    // Then: the four rows moved, and nothing else; each moved by the
    // scroll alone, which the a11y mirror leaves out (R6)
    assert_eq!((counts.boxes_moved, counts.boxes_scrolled), (4, 4));
}

#[test]
fn a_row_moved_by_a_change_above_it_is_not_counted_as_scrolled() {
    // Given
    let (mut doc, texts) = rows();

    // When
    doc.set_text(texts[1], "a much longer label that wraps in 100px");
    let counts = updated(&mut doc, 200.0);

    // Then
    assert_eq!(counts.boxes_scrolled, 0);
}

#[test]
fn a_scrolled_row_laid_out_again_is_moved_not_scrolled() {
    // Given: a scroll container of four rows, scrolled, with the
    // second row made taller in the same update
    let mut list = list("overflow: auto;");
    updated(&mut list.doc, 200.0);

    // When
    assert!(list.doc.scroll_node_by(list.container, [0.0, 30.0]));
    list.doc
        .set_attribute(list.rows[1], "style", "height: 60px");
    let counts = updated(&mut list.doc, 200.0);

    // Then: the rows after it are laid out again, so they are moved,
    // not scrolled, whatever the scroll did to them
    assert!(counts.boxes_scrolled <= 1, "{counts:?}");
    assert!(counts.boxes_moved >= 3, "{counts:?}");
}

#[test]
fn a_box_made_absolute_places_its_children_against_its_new_place() {
    // Given: a box holding a row, in flow inside a relative holder
    let (mut doc, _) = rows();
    let body = doc.body();
    let holder = element(&mut doc, body, "div", "holder");
    let moving = element(&mut doc, holder, "div", "");
    let child = element(&mut doc, moving, "div", "row");
    text(&mut doc, child, "a");
    updated(&mut doc, 200.0);

    // When: the box is made absolute, 100px down the holder
    doc.set_attribute(moving, "class", "abs");
    updated(&mut doc, 200.0);

    // Then: its child sits at the box's new corner
    let painted = doc.painted().expect("painted");
    let holder_box = painted.border_box(holder).expect("a box");
    let child_box = painted.border_box(child).expect("a box");
    assert_eq!(
        (child_box.x, child_box.y),
        (holder_box.x + 20.0, holder_box.y + 100.0)
    );
}

/// The first row's top after a scroll of 30px, in a scroller given,
/// before the scroll, the class that `restyled` names (which changes no
/// layout) on the node it names.
fn top_after_restyle_and_scroll(
    restyled: fn(&support::scroll_list::List) -> (NodeId, &'static str),
) -> f32 {
    let mut list = list("overflow: auto;");
    list.doc
        .add_stylesheet("tint.css", ".tint { color: red; }")
        .expect("parses");
    updated(&mut list.doc, 200.0);
    let (node, class) = restyled(&list);
    list.doc.set_attribute(node, "class", class);
    updated(&mut list.doc, 200.0);
    assert!(list.doc.scroll_node_by(list.container, [0.0, 30.0]));
    updated(&mut list.doc, 200.0);
    let painted = list.doc.painted().expect("painted");
    painted.border_box(list.rows[0]).expect("a box").y
}

#[test]
fn a_scroll_after_a_restyle_of_the_scroller_that_changes_no_layout_moves_the_rows() {
    // Given, When
    let top = top_after_restyle_and_scroll(|list| (list.container, "list tint"));

    // Then
    assert_eq!(top, -30.0);
}

#[test]
fn a_scroll_after_a_restyle_of_an_ancestor_that_changes_no_layout_moves_the_rows() {
    // Given, When
    let top = top_after_restyle_and_scroll(|list| (list.doc.body(), "tint"));

    // Then
    assert_eq!(top, -30.0);
}
