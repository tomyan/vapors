//! Scroll offsets (F2c S9): clamped to the content, which counts end
//! margins and padding and not out-of-flow boxes placed outside;
//! applied in whole device pixels; kept between updates; set by
//! `Document::scroll_to`.

mod support;

use support::element;
use support::scroll_list::{at, list, scroll, top};
use vapors_dom::Dom;

#[test]
fn the_offset_stops_at_the_end_of_the_content_and_at_the_start() {
    // Given
    let mut list = list("overflow: auto;");

    // When / Then
    scroll(&mut list.doc, [0.0, 1000.0]);
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 110.0]));
    scroll(&mut list.doc, [0.0, -1000.0]);
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 0.0]));
}

#[test]
fn the_offset_is_applied_in_whole_device_pixels() {
    // Given
    let mut list = list("overflow: auto;");
    list.doc.update(at(2.0)).expect("updates");

    // When: 10.3 CSS px is 20.6 device px
    list.doc.scroll_by([10.0, 10.0], [0.0, 10.3]);
    let row = list
        .doc
        .update(at(2.0))
        .expect("updates")
        .border_box(list.rows[1])
        .expect("a box");

    // Then: 21 device px, and the offset itself is kept unrounded
    assert_eq!(row.y, 40.0 - 10.5);
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 10.3]));
}

#[test]
fn the_offset_is_kept_between_updates_and_clamped_when_the_content_shrinks() {
    // Given: scrolled to the end
    let mut list = list("overflow: auto;");
    scroll(&mut list.doc, [0.0, 1000.0]);
    assert_eq!(top(&mut list.doc, list.rows[3]), 10.0);

    // When: two rows go; 80 px of content leaves 30 px to scroll
    list.doc.remove(list.rows[3]);
    list.doc.remove(list.rows[2]);
    list.doc.update(at(1.0)).expect("updates");

    // Then
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 30.0]));
    assert_eq!(top(&mut list.doc, list.rows[1]), 10.0);
}

#[test]
fn an_element_that_stops_scrolling_drops_its_offset() {
    // Given: scrolled
    let mut list = list("");
    list.doc
        .set_attribute(list.container, "style", "overflow: auto;");
    scroll(&mut list.doc, [0.0, 30.0]);

    // When: no longer a scroll container
    list.doc.set_attribute(list.container, "style", "");

    // Then
    assert_eq!(top(&mut list.doc, list.rows[1]), 40.0);
    assert_eq!(list.doc.scroll_offset(list.container), None);
}

#[test]
fn scroll_to_sets_the_offset_within_the_range() {
    // Given
    let mut list = list("overflow: auto;");
    list.doc.update(at(1.0)).expect("updates");

    // When / Then
    assert!(list.doc.scroll_to(list.container, [0.0, 40.0]));
    assert_eq!(top(&mut list.doc, list.rows[1]), 0.0);
    assert!(list.doc.scroll_to(list.container, [0.0, 20.0]));
    assert_eq!(top(&mut list.doc, list.rows[1]), 20.0);
    assert!(list.doc.scroll_to(list.container, [5.0, 500.0]));
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 110.0]));
    assert!(
        !list.doc.scroll_to(list.rows[0], [0.0, 10.0]),
        "not a scroll container"
    );
}

#[test]
fn an_out_of_flow_child_placed_outside_does_not_lengthen_the_scroll() {
    // Given: an absolute box 500 px down, placed by the body
    let mut list = list("overflow: auto;");
    element(&mut list.doc, list.container, "div", "far");
    list.doc
        .add_stylesheet(
            "far.css",
            ".far { position: absolute; top: 500px; width: 5px; height: 5px; }",
        )
        .expect("parses");

    // When
    scroll(&mut list.doc, [0.0, 1000.0]);

    // Then: still the rows' 110 px
    assert_eq!(list.doc.scroll_offset(list.container), Some([0.0, 110.0]));
}
