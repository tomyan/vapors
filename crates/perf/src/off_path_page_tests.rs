//! The changes an off-path page makes: an edit of the middle unit's
//! value, the body's scroll and rewind, a unit's own scroll and rewind,
//! and a unit inserted and removed.

use super::{UnitPage, page_viewport};
use crate::off_path_scrollers::CODE_BLOCKS;

fn page(count: usize) -> UnitPage {
    UnitPage::new(&CODE_BLOCKS, count).expect("the code blocks page mounts")
}

fn updated(page: &UnitPage) {
    page.doc
        .borrow_mut()
        .update(page_viewport())
        .expect("updates");
}

fn body_offset(page: &UnitPage) -> f32 {
    let doc = page.doc.borrow();
    doc.scroll_offset(doc.body()).expect("the body scrolls")[1]
}

#[test]
fn an_edit_sets_the_middle_units_value_to_another_five_digit_text() {
    // Given
    let page = page(10);
    let value = page.units[5].value;

    // When
    page.edit(3);

    // Then
    assert_eq!(page.doc.borrow().text(value), Some("90003"));
}

#[test]
fn the_body_scrolls_down_by_the_step_and_back_to_the_top_at_its_end() {
    // Given: 3 short messages fit in the viewport but 40 do not
    let page = page(40);

    // When
    page.scroll_body(24.0);
    updated(&page);

    // Then
    assert!((body_offset(&page) - 24.0).abs() < 0.01);
    for _ in 0..1000 {
        page.scroll_body(24.0);
        updated(&page);
        if body_offset(&page) == 0.0 {
            return;
        }
    }
    panic!("the body never scrolled back to its top");
}

#[test]
fn the_second_units_own_scroller_moves_right_by_the_step() {
    // Given
    let page = page(10);
    let inner = page.units[1].inner.expect("a code block");

    // When
    page.scroll_inner(24.0);
    updated(&page);

    // Then
    let offset = page.doc.borrow().scroll_offset(inner).expect("it scrolls");
    assert_eq!(offset, [24.0, 0.0]);
}

#[test]
fn insert_adds_a_unit_before_the_middle_unit_then_removes_it() {
    // Given
    let mut page = page(10);
    let middle = page.units[5].element;
    let count = |page: &UnitPage| page.doc.borrow().children(page.container).len();

    // When
    page.toggle_inserted();

    // Then
    assert_eq!(count(&page), 11);
    assert_eq!(page.doc.borrow().children(page.container)[6], middle);
    page.toggle_inserted();
    assert_eq!(count(&page), 10);
}
