//! Lanes whose children cannot be frames (`docs/plans/splice-layout.md`,
//! section 11.1): a start that is not whole device pixels at the scale,
//! or a scroll container in a row, however it got there, is counted and
//! the lane keeps the placement from before; a lane whose starts are
//! whole at the scale is framed. Each case matches a full update.

mod support;

use support::lane_page::header_list;
use support::narrow_list::ROWS;
use support::viewport;
use vapors_dom::Dom;
use vapors_elements::{LaneDecline, Viewport};

/// The header a whole 10px, so body's lane does not collapse through it.
const HEAD: &str = "body { margin: 0; } .head { height: 10px; }";

/// The view at `scale`.
fn view(scale: f32) -> Viewport {
    Viewport {
        scale,
        ..viewport(400.0, 300.0)
    }
}

#[test]
fn a_list_with_a_fractional_row_is_counted_and_placed_as_before() {
    // Given: one row a fraction of a pixel taller, so later starts are
    // not whole device pixels
    let page = header_list(
        100,
        &format!("{ROWS} {HEAD} .odd {{ height: 20.5px; }}"),
        view(1.0),
    );

    // When
    let counts = page.edited(view(1.0), |page| {
        page.doc
            .borrow_mut()
            .set_attribute(page.rows[10], "class", "row odd");
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
    let page = header_list(
        100,
        &format!("{ROWS} {HEAD} .scroll {{ overflow: auto; }}"),
        view(1.0),
    );

    // When
    let counts = page.edited(view(1.0), |page| {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[10])[0];
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
fn a_scroller_put_in_a_fixed_size_cell_is_counted_and_placed_as_before() {
    // Given: a cell of a fixed size holding a label, which gives its
    // baseline, and a spacer; a cell given a new child that answers as
    // before is laid out alone, and no row is laid out again
    let css = format!(
        "{ROWS} {HEAD} .cell {{ height: 20px; display: flex; flex-direction: column; }} \
         .label, .spacer {{ width: 40px; height: 10px; }} .scroll {{ overflow: auto; }}"
    );
    let page = header_list(100, &css, view(1.0));
    let spacer = {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[10])[0];
        let text = doc.children(cell)[0];
        doc.remove(text);
        let label = support::element(&mut doc, cell, "div", "label");
        support::text(&mut doc, label, "x");
        let spacer = support::element(&mut doc, cell, "div", "spacer");
        doc.update(view(1.0)).expect("updates");
        spacer
    };

    // When: the spacer replaced by a scroll container of its size
    let counts = page.edited(view(1.0), |page| {
        let mut doc = page.doc.borrow_mut();
        let cell = doc.children(page.rows[10])[0];
        doc.remove(spacer);
        support::element(&mut doc, cell, "div", "spacer scroll");
    });

    // Then: the cell laid out alone, and the lane learns of the scroll
    // container, counts it and keeps the placement
    assert_eq!(counts.owners_isolated, 1, "{counts:?}");
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFrameScroller),
        "{counts:?}"
    );
}

#[test]
fn rows_whole_at_scale_2_are_framed() {
    // Given: rows 20.5px tall, 41 device pixels at scale 2
    let page = header_list(
        100,
        &format!("{ROWS} {HEAD} .row {{ height: 20.5px; }}"),
        view(2.0),
    );

    // When
    let counts = page.edited(view(2.0), |page| _ = page.insert_at(50));

    // Then: framed; the list, body and html grow and no row moves
    assert_eq!(counts.lanes_unframed, 0, "{counts:?}");
    assert!(counts.boxes_moved <= 3, "{counts:?}");
}

#[test]
fn rows_fractional_at_scale_1_5_are_counted_and_placed_as_before() {
    // Given: rows 20.5px tall, 30.75 device pixels at scale 1.5
    let page = header_list(
        100,
        &format!("{ROWS} {HEAD} .row {{ height: 20.5px; }}"),
        view(1.5),
    );

    // When
    let counts = page.edited(view(1.5), |page| _ = page.insert_at(50));

    // Then
    assert_eq!(
        counts.lane_unframed,
        Some(LaneDecline::LaneFramePixel),
        "{counts:?}"
    );
    assert!(counts.boxes_moved > 50, "{counts:?}");
}
