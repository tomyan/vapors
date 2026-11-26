//! `Document::update_frame` (virtual list V4) under a resting pointer and
//! a clamped offset: the hover a `resize` handler's change moves is
//! drawn in the same frame, by a third update only such a frame runs;
//! a hover style that moves boxes is still refused; a container fires at
//! most one `scroll` a frame, and the offset its layout clamped reaches
//! a `resize` handler on the `resize` itself.

use crate::support;

use support::frame_page::{OnResize, page};
use support::pointer::quad_colours;
use vapors_elements::Document;

const HOVERED: [u8; 4] = [0, 255, 0, 255];

/// The hover of a row a resize adds shows green.
const ADDED_HOVER: &str = ".added:hover { background-color: #00ff00; }";

/// A row that moves out from under the pointer while hovered, and back
/// when not: a hover that never settles.
const MOVING_HOVER: &str = ".row:hover { margin-top: 200px; }";

#[test]
fn a_resting_pointer_over_a_row_a_resize_adds_hovers_it_in_that_frame() {
    // Given: the pointer resting on the list's first row
    let page = page(OnResize::AddsARow, ADDED_HOVER);
    page.frame(200.0);
    Document::pointer_moved(&page.doc, [10.0, 15.0]);
    page.frame(200.0);

    // When: the window taller, a row added under the pointer
    let did = page.frame(300.0);

    // Then: the third update moved the hover onto it
    assert_eq!(did, ["update", "resize", "update", "update"]);
    assert_eq!(page.class_at(15.0).as_deref(), Some("added"));
    assert!(quad_colours(&page.doc).contains(&HOVERED));
}

#[test]
fn a_pointer_resting_before_the_first_frame_hovers_the_row_its_resize_adds() {
    // Given: the pointer over where the list will be
    let page = page(OnResize::AddsARow, ADDED_HOVER);
    Document::pointer_moved(&page.doc, [10.0, 15.0]);

    // When
    let did = page.frame(200.0);

    // Then
    assert_eq!(did, ["update", "resize", "update", "update"]);
    assert!(quad_colours(&page.doc).contains(&HOVERED));
}

#[test]
fn a_hover_that_moves_boxes_is_refused_after_two_updates_without_a_resize() {
    // Given: settled, then the pointer onto the first row
    let page = page(OnResize::Nothing, MOVING_HOVER);
    page.frame(200.0);
    Document::pointer_moved(&page.doc, [10.0, 15.0]);

    // When
    let refused = page.try_frame(200.0);

    // Then
    let message = refused.expect_err("a hover that never settles is refused");
    assert!(message.contains("each of two updates"), "{message}");
}

#[test]
fn a_hover_that_moves_boxes_is_refused_after_three_updates_with_a_resize() {
    // Given: the pointer resting on the first row before the first frame
    let page = page(OnResize::AddsARow, MOVING_HOVER);
    Document::pointer_moved(&page.doc, [10.0, 55.0]);

    // When
    let refused = page.try_frame(200.0);

    // Then
    let message = refused.expect_err("a hover that never settles is refused");
    assert!(message.contains("each of three updates"), "{message}");
}

#[test]
fn a_scroll_and_a_clamp_in_one_frame_fire_one_scroll() {
    // Given: at the end of the 90 px list, 70 px down, then scrolled there again
    let page = page(OnResize::Nothing, "");
    page.frame(100.0);
    let list = page.list;
    assert!(page.doc.borrow_mut().scroll_to(list, [0.0, 70.0]));

    // When: 150 px tall, a 140 px list with 20 px to scroll
    let did = page.frame(150.0);

    // Then: the scroll of 70 only; the resize carries the clamped 20
    assert_eq!(did, ["scroll", "update", "resize"]);
    assert_eq!(
        page.resize_offsets.borrow().last(),
        Some(&Some([0.0, 20.0]))
    );
}

#[test]
fn the_offset_a_layout_clamped_fires_its_scroll_in_the_next_frame() {
    // Given: at the end of the 90 px list, then made taller
    let page = page(OnResize::Nothing, "");
    page.frame(100.0);
    let list = page.list;
    assert!(page.doc.borrow_mut().scroll_to(list, [0.0, 70.0]));
    page.frame(100.0);
    page.frame(150.0);

    // When
    let did = page.frame(150.0);

    // Then: the clamp's scroll, once
    assert_eq!(did, ["scroll", "update"]);
    assert_eq!(page.doc.borrow().scroll_offset(list), Some([0.0, 20.0]));
}

#[test]
fn a_scroll_a_resize_handler_makes_fires_in_the_next_frame() {
    // Given: a resize handler that scrolls its list
    let page = page(OnResize::Scrolls, "");
    let did = page.frame(100.0);
    assert_eq!(did, ["update", "resize", "update"]);

    // When
    let did = page.frame(100.0);

    // Then: its scroll, once, at the next frame's start
    assert_eq!(did, ["scroll", "update"]);
    assert_eq!(
        page.doc.borrow().scroll_offset(page.list),
        Some([0.0, 30.0])
    );
}
