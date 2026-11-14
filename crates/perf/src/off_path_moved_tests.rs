//! The moved subtree sits at its fractional offset, each animation step
//! moves it to its entry of OFFSETS, the step after the last returns to
//! the first, and Reclass moves nothing.

use super::{OFFSETS, TRANSLATED, animate, reclass};
use crate::off_path_page::{UnitPage, page_viewport};

/// The first unit's border box origin after an update.
fn first_origin(page: &UnitPage) -> (f32, f32) {
    let mut doc = page.doc.borrow_mut();
    let painted = doc.update(page_viewport()).expect("updates");
    let rect = painted.border_box(page.units[0].element).expect("a box");
    (rect.x, rect.y)
}

fn page() -> UnitPage {
    UnitPage::new(&TRANSLATED, 20).expect("mounts")
}

#[test]
fn the_subtree_sits_at_the_first_offset() {
    assert_eq!(first_origin(&page()), OFFSETS[0]);
}

#[test]
fn every_animation_step_moves_the_subtree_to_its_offset_and_away_from_the_last() {
    let page = page();
    for (step, offset) in OFFSETS.iter().enumerate().skip(1) {
        animate(&page, step);
        assert_eq!(first_origin(&page), *offset, "step {step}");
        assert_ne!(OFFSETS[step - 1], *offset, "step {step}");
    }
}

#[test]
fn the_step_after_the_last_returns_to_the_first_offset() {
    // Given
    let page = page();
    animate(&page, OFFSETS.len() - 1);

    // When
    animate(&page, OFFSETS.len());

    // Then
    assert_eq!(first_origin(&page), OFFSETS[0]);
}

#[test]
fn reclass_changes_the_class_and_moves_nothing() {
    // Given
    let page = page();

    // When
    reclass(&page, 0);

    // Then
    assert_eq!(first_origin(&page), OFFSETS[0]);
    let class = page
        .doc
        .borrow()
        .attribute(page.container, "class")
        .map(str::to_owned);
    assert_eq!(class.as_deref(), Some("moved unmatched"));
}
