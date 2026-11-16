//! Lanes whose children cannot be frames (`docs/plans/splice-layout.md`,
//! section 11.1): a start that is not whole device pixels at the scale
//! is counted and the lane keeps the placement from before; a lane
//! whose starts are whole at the scale is framed. A scroll container in
//! a row no longer declines (`lane_scrollers.rs`). Each case matches a
//! full update.

use crate::support;

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
