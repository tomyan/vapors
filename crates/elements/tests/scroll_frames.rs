//! Scroll frames (IU4, `docs/plans/iu4.md` section 3): a part inside a
//! scroll container is kept in the container's frame and put on the
//! page as it is read, so its border box, hits and items are where it
//! shows. A debug build checks every update's frames and each
//! element's frame against a fresh paint (`kept_paint_check.rs`).

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::oracle::assert_matches_full;
use support::scroll_list::{List, at, list};
use vapors_dom::Dom;
use vapors_elements::Painted;
use vapors_scene::{Color, Item, Rect};

/// The rows' colour.
const ROW: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// The list of `support::scroll_list` with blue rows, its container
/// styled `list_css`, scrolled `scrolled` px down.
fn scrolled(list_css: &str, scrolled: f32) -> List {
    let mut list = list(list_css);
    list.doc
        .add_stylesheet("rows.css", ".row { background-color: blue; }")
        .expect("parses");
    list.doc.update(at(1.0)).expect("updates");
    list.doc.scroll_node_by(list.container, [0.0, scrolled]);
    list.doc.update(at(1.0)).expect("updates");
    list
}

/// The rows' quads on the page, each with its clip.
fn row_quads(painted: &Painted) -> Vec<(Rect, Option<Rect>)> {
    let items = painted.scene().items();
    let rows = items.filter_map(|(item, clip)| match item {
        Item::Quad(quad) if quad.background == ROW => Some((quad.rect, clip)),
        _ => None,
    });
    rows.collect()
}

#[test]
fn a_scrolled_rows_border_box_is_where_it_shows() {
    // Given: 40 px rows scrolled 30 px
    let list = scrolled("overflow: auto;", 30.0);

    // When
    let painted = list.doc.painted().expect("painted");
    let second = painted.border_box(list.rows[1]).expect("a box");

    // Then
    assert_eq!((second.y, second.height), (10.0, 40.0));
    assert_eq!(painted.padding_origin(list.rows[1]), Some([0.0, 10.0]));
}

#[test]
fn a_scrolled_row_takes_the_pointer_where_it_shows_and_not_past_the_list() {
    // Given
    let list = scrolled("overflow: auto;", 30.0);

    // When
    let painted = list.doc.painted().expect("painted");

    // Then: the second row shows from 10 to 50; the third, from 50 on,
    // is cut off by the list, which ends at 50, as the page does
    assert_eq!(painted.hit([50.0, 12.0]), Some(list.rows[1]));
    assert_eq!(painted.hit([50.0, 55.0]), None);
}

#[test]
fn a_scrolled_rows_items_are_drawn_where_it_shows_under_the_lists_clip() {
    // Given
    let list = scrolled("overflow: auto;", 30.0);

    // When
    let quads = row_quads(list.doc.painted().expect("painted"));

    // Then
    let clip = Some(Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(quads[0], (Rect::new(0.0, -30.0, 100.0, 10.0), clip));
    assert_eq!(quads[1], (Rect::new(0.0, 10.0, 100.0, 50.0), clip));
}

#[test]
fn a_list_that_starts_to_scroll_paints_its_rows_as_a_full_paint_does() {
    // Given: the list clips but does not scroll
    let list = scrolled("overflow: hidden;", 0.0);
    let doc = Rc::new(RefCell::new(list.doc));
    doc.borrow_mut()
        .add_stylesheet("scroller.css", ".list.scrolls { overflow: auto; }")
        .expect("parses");
    doc.borrow_mut().update(at(1.0)).expect("updates");

    // When: it scrolls, its rows' frame changes
    doc.borrow_mut()
        .set_attribute(list.container, "class", "list scrolls");

    // Then
    assert_matches_full(&doc, at(1.0), "a list that starts to scroll");
}

#[test]
fn a_row_on_a_half_device_pixel_scrolled_across_the_top_snaps_as_the_page_does() {
    // Given: 25 px rows at 1.5x, so the second's top is 37.5 device px
    let mut list = list("overflow: auto;");
    list.doc
        .add_stylesheet("rows.css", ".row { height: 25px; }")
        .expect("parses");
    let at_1_5 = vapors_elements::Viewport {
        scale: 1.5,
        ..at(1.0)
    };
    list.doc.update(at_1_5).expect("updates");

    // When: scrolled 26 px, 39 device px, the second row's top is at
    // -1.5 device px on the page
    list.doc.scroll_node_by(list.container, [0.0, 26.0]);
    list.doc.update(at_1_5).expect("updates");

    // Then: the placement check (debug builds) holds, and the row is
    // where it is on the page
    let second = list
        .doc
        .painted()
        .expect("painted")
        .border_box(list.rows[1]);
    assert_eq!(second.map(|rect| rect.y), Some(-1.0));
}

#[test]
fn a_scroll_not_whole_in_css_pixels_puts_a_half_device_pixel_edge_where_paint_does() {
    // Given: 25 px rows at 1.5x, the third's bottom at 112.5 device px
    let mut list = list("overflow: auto;");
    let rows_css = ".row { height: 25px; background-color: blue; }";
    list.doc
        .add_stylesheet("rows.css", rows_css)
        .expect("parses");
    let at_1_5 = vapors_elements::Viewport {
        scale: 1.5,
        ..at(1.0)
    };
    list.doc.update(at_1_5).expect("updates");

    // When: scrolled 0.5 px, applied as 1 device px, which is no f32
    // number of CSS px (0.6666667)
    list.doc.scroll_node_by(list.container, [0.0, 0.5]);
    list.doc.update(at_1_5).expect("updates");

    // Then: the placement check (debug builds) holds, and the third
    // row's bottom snaps to 113 device px less the 1 scrolled
    let quads = row_quads(list.doc.painted().expect("painted"));
    let third = quads[2].0;
    assert_eq!((third.top, third.bottom), (74.0, 112.0));
}

#[test]
fn a_draw_after_a_scroll_batches_the_rows_where_they_show() {
    // Given: a draw, which keeps its batches
    let mut list = scrolled("overflow: auto;", 0.0);
    let view = Rect::new(0.0, 0.0, 200.0, 200.0);
    drop(list.doc.painted().expect("painted").batches(view));

    // When: the list scrolls, which paints no part
    list.doc.scroll_node_by(list.container, [0.0, 30.0]);
    let painted = list.doc.update(at(1.0)).expect("updates");

    // Then: the batches are made again, with the rows where they show
    // in the list's frame (a debug build compares them with the whole
    // scene's)
    let kept = painted.kept_batches(view).clone();
    assert_eq!(
        (kept.batched, kept.frames),
        painted.scene().framed_batches(view)
    );
    let quads = row_quads(painted);
    assert_eq!(quads[1].0, Rect::new(0.0, 10.0, 100.0, 50.0));
}
