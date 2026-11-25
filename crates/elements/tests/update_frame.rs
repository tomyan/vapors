//! `Document::update_frame` (virtual list V4, `docs/plans/virtual-list.md`):
//! a frame's Update phase fires the `scroll`s owed before its first
//! update, at most one per container, and the `resize`s that update owes
//! after it, each carrying the container's offset as laid out; what a
//! `resize` handler changes the second update lays out, so the frame
//! whose layout changed a size draws it. The order and the bound.

use crate::support;

use support::frame_page::{OnResize, page};

#[test]
fn the_first_frame_fires_the_resize_after_its_update() {
    // Given
    let page = page(OnResize::Nothing, "");

    // When
    let did = page.frame(200.0);

    // Then: the handler changed nothing, so one update
    assert_eq!(did, ["update", "resize"]);
}

#[test]
fn what_a_resize_handler_changes_is_drawn_in_the_frame_of_the_resize() {
    // Given
    let page = page(OnResize::AddsARow, "");

    // When
    let did = page.frame(200.0);

    // Then: the second update laid the added row out at the list's top
    assert_eq!(did, ["update", "resize", "update"]);
    assert_eq!(page.class_at(15.0).as_deref(), Some("added"));
}

#[test]
fn a_window_resize_is_drawn_in_its_own_frame() {
    // Given: settled at 200 px, one row added
    let page = page(OnResize::AddsARow, "");
    page.frame(200.0);

    // When
    let did = page.frame(300.0);

    // Then: a second row added and drawn in the same frame
    assert_eq!(did, ["update", "resize", "update"]);
    assert_eq!(page.class_at(55.0).as_deref(), Some("added"));
}

#[test]
fn a_scroll_fires_before_the_update_and_the_resize_after_it() {
    // Given: settled at 100 px, 160 px of rows in 90, one added
    let page = page(OnResize::AddsARow, "");
    page.frame(100.0);
    let list = page.list;
    assert!(page.doc.borrow_mut().scroll_to(list, [0.0, 30.0]));

    // When: scrolled, and the window 10 px taller, which keeps 30 px in range
    let did = page.frame(110.0);

    // Then
    assert_eq!(did, ["scroll", "update", "resize", "update"]);
}

#[test]
fn a_settled_frame_updates_once_and_asks_for_no_other() {
    // Given
    let page = page(OnResize::AddsARow, "");
    page.frame(200.0);
    page.doc.borrow_mut().take_changed();

    // When
    let did = page.frame(200.0);

    // Then
    assert_eq!(did, ["update"]);
    assert!(!page.doc.borrow_mut().take_changed());
}

#[test]
fn a_size_that_never_settles_runs_two_updates_a_frame_and_owes_the_next_resize() {
    // Given: a handler that makes its list taller at each resize
    let page = page(OnResize::Grows, "");

    // When
    let did = page.frame(200.0);

    // Then: the frame ends after its second update; the next frame owes a resize
    assert_eq!(did, ["update", "resize", "update"]);
    assert!(page.doc.borrow_mut().take_changed());
}

#[test]
fn the_frame_after_one_that_did_not_settle_still_runs_two_updates() {
    // Given
    let page = page(OnResize::Grows, "");
    page.frame(200.0);

    // When
    let did = page.frame(200.0);

    // Then: the resize owed fires at the frame's start, the next after its first update
    assert_eq!(did, ["resize", "update", "resize", "update"]);
}
