//! The moved subtree sits at its fractional offset, and each animation
//! step moves it to the next.

use super::{TRANSLATED, animate};
use crate::off_path_page::{UnitPage, page_viewport};

/// The first unit's border box origin after an update.
fn first_origin(page: &UnitPage) -> (f32, f32) {
    let mut doc = page.doc.borrow_mut();
    let painted = doc.update(page_viewport()).expect("updates");
    let rect = painted.border_box(page.units[0].element).expect("a box");
    (rect.x, rect.y)
}

#[test]
fn the_subtree_sits_at_its_fractional_offset() {
    let page = UnitPage::new(&TRANSLATED, 20).expect("mounts");
    assert_eq!(first_origin(&page), (0.5, 0.25));
}

#[test]
fn each_animation_step_moves_the_subtree_to_the_next_offset_and_wraps() {
    // Given
    let page = UnitPage::new(&TRANSLATED, 20).expect("mounts");

    // When
    animate(&page, 3);

    // Then
    assert_eq!(first_origin(&page), (1.125, 0.375));
    animate(&page, 8);
    assert_eq!(first_origin(&page), (0.5, 0.25));
}
