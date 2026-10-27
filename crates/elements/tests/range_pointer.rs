//! A range input by pointer (3-E3a): a press sets the value where the
//! thumb's centre would be, captures the pointer and drags; `input`
//! fires on each change and `change` once at the drag's end; a disabled
//! range takes nothing. The order against Chromium is pinned by the
//! corpus's `range.events`; these pin the values and the edges.

mod support;

use support::range_page::{key, page, page_with_second, value};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;

#[test]
fn a_press_sets_the_value_where_the_thumbs_centre_lands() {
    // Given: the thumb is 16 px, so x runs from 8 to 192 over 0..100
    let page = page();

    // When
    Document::pointer_pressed(&page.doc, [8.0, 10.0]);
    Document::pointer_released(&page.doc, [8.0, 10.0]);
    let at_start = value(&page, page.level);
    Document::pointer_pressed(&page.doc, [100.0, 10.0]);
    Document::pointer_released(&page.doc, [100.0, 10.0]);

    // Then
    assert_eq!(at_start, "0");
    assert_eq!(value(&page, page.level), "50");
    assert_eq!(
        *page.log.borrow(),
        ["input 0 0", "change 0 0", "input 50 50", "change 50 50"]
    );
}

#[test]
fn a_drag_follows_the_pointer_past_the_range_and_changes_once_at_its_end() {
    // Given
    let page = page();

    // When: pressed at 50, dragged to 77, then off the page
    Document::pointer_pressed(&page.doc, [100.0, 10.0]);
    Document::pointer_moved(&page.doc, [150.0, 10.0]);
    Document::pointer_moved(&page.doc, [900.0, 300.0]);
    Document::pointer_released(&page.doc, [900.0, 300.0]);

    // Then: the capturing range heard every move; one change, at the end
    assert_eq!(
        *page.log.borrow(),
        [
            "input 50 50",
            "input 77 77",
            "input 100 100",
            "change 100 100"
        ]
    );
}

#[test]
fn a_release_at_the_value_the_press_began_with_fires_no_change() {
    // Given: at 20, which x 44.8 maps to
    let page = page();

    // When
    Document::pointer_pressed(&page.doc, [44.8, 10.0]);
    Document::pointer_released(&page.doc, [44.8, 10.0]);

    // Then
    assert_eq!(value(&page, page.level), "20");
    assert!(page.log.borrow().is_empty(), "{:?}", page.log.borrow());
}

#[test]
fn a_prevented_pointerdown_leaves_the_value_alone() {
    // Given
    let page = page();
    page.doc.borrow_mut().listen(
        page.level,
        "pointerdown",
        Box::new(|e: &mut Event| e.prevent_default()),
    );

    // When
    Document::pointer_pressed(&page.doc, [100.0, 10.0]);
    Document::pointer_moved(&page.doc, [150.0, 10.0]);
    Document::pointer_released(&page.doc, [150.0, 10.0]);

    // Then
    assert_eq!(value(&page, page.level), "20");
    assert!(page.log.borrow().is_empty());
}

#[test]
fn a_disabled_range_takes_no_press_and_no_key() {
    // Given: `off` at y 20..40, disabled
    let page = page();

    // When
    Document::pointer_pressed(&page.doc, [150.0, 30.0]);
    Document::pointer_released(&page.doc, [150.0, 30.0]);

    // Then: unchanged, and not focused
    assert_eq!(value(&page, page.off), "50");
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_press_focuses_the_range_without_the_ring_and_a_key_then_shows_it() {
    // Given
    let page = page();

    // When
    Document::pointer_pressed(&page.doc, [100.0, 10.0]);
    Document::pointer_released(&page.doc, [100.0, 10.0]);
    let after_press = page.doc.borrow().focus_visible();
    key(&page, "ArrowRight");

    // Then
    assert_eq!(page.doc.borrow().active_element(), Some(page.level));
    assert!(!after_press, "a press shows no ring");
    assert!(page.doc.borrow().focus_visible(), "a key does");
}

#[test]
fn two_fingers_drag_two_ranges_each_on_its_own() {
    // Given: a second live range below the first
    let page = page_with_second();

    // When: finger 0 drags `level`, finger 1 presses `second` meanwhile,
    // then each lifts
    Document::touch_pressed(&page.doc, 0, [100.0, 10.0]);
    Document::touch_moved(&page.doc, 0, [150.0, 10.0]);
    Document::touch_pressed(&page.doc, 1, [50.0, 30.0]);
    Document::touch_moved(&page.doc, 0, [190.0, 10.0]);
    Document::touch_released(&page.doc, 0, [190.0, 10.0]);
    Document::touch_released(&page.doc, 1, [50.0, 30.0]);

    // Then: `level` kept following finger 0 and changed once at its end,
    // and the second range took finger 1's press
    assert_eq!(
        *page.log.borrow(),
        ["input 50 50", "input 77 77", "input 99 99", "change 99 99"]
    );
    assert_eq!(value(&page, page.off), "23");
}

#[test]
fn a_finger_moved_across_the_range_is_the_pans_and_slides_no_more() {
    // Given
    let page = page();

    // When: down at 50, then more down the page than along, then lifted
    Document::touch_pressed(&page.doc, 0, [100.0, 10.0]);
    Document::touch_moved(&page.doc, 0, [110.0, 40.0]);
    let owned = page.doc.borrow().touch_owned(0);
    let may_scroll = Document::touch_panned(&page.doc);
    let before_lift = page.log.borrow().len();
    Document::touch_released(&page.doc, 0, [110.0, 40.0]);

    // Then: not owned, so the pan scrolls and cancels the touch; the
    // sideways 10 px did not move the value, and the change from 20 to
    // 50 fires at the lift, not at the cancel
    assert!(!owned && may_scroll);
    assert_eq!(before_lift, 1);
    assert_eq!(*page.log.borrow(), ["input 50 50", "change 50 50"]);
    assert_eq!(value(&page, page.level), "50");
}

#[test]
fn a_finger_drags_a_range_too() {
    // Given
    let page = page();

    // When
    Document::touch_pressed(&page.doc, 0, [100.0, 10.0]);
    Document::touch_moved(&page.doc, 0, [150.0, 10.0]);
    let owned = page.doc.borrow().touch_owned(0);
    let may_scroll = Document::touch_panned(&page.doc);
    Document::touch_released(&page.doc, 0, [150.0, 10.0]);

    // Then: the touch is owned by the range while down, so a pan does
    // not scroll or cancel it
    assert!(owned && !may_scroll);
    assert_eq!(
        *page.log.borrow(),
        ["input 50 50", "input 77 77", "change 77 77"]
    );
}
