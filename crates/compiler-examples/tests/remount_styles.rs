//! Every mount of a styled component calls `add_style` and
//! `declare_events`. A second mount of a type already mounted adds no
//! sheet and declares no new event, so the next update restyles and
//! builds only what the mount added; a mount of a type whose sheet is
//! new restyles every element, as any new sheet does.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::styled_card::{StyledCard, StyledCardProps};
use vapors_compiler_examples::styled_title::{StyledTitle, StyledTitleProps};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, UpdateCounts, Viewport};
use vapors_text::FontSystem;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 200.0,
    scale: 1.0,
};

/// A document with one `StyledCard` (3 elements, 2 text runs) in its
/// body, updated.
fn one_card() -> Rc<RefCell<Document>> {
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let _card = StyledCard::mount(&doc, body, StyledCardProps::default());
    doc.borrow_mut().update(VIEWPORT).expect("updates");
    doc
}

fn update_counts(doc: &Rc<RefCell<Document>>) -> UpdateCounts {
    doc.borrow_mut().update(VIEWPORT).expect("updates");
    doc.borrow().update_counts()
}

#[test]
fn a_second_mount_of_a_mounted_type_restyles_only_its_own_elements() {
    // Given
    let doc = one_card();
    let body = doc.borrow().body();

    // When
    let _second = StyledCard::mount(&doc, body, StyledCardProps::default());
    let counts = update_counts(&doc);

    // Then: the new card's section, h2 and p, and none of the first's
    assert_eq!(counts.elements_restyled, 3);
}

/// A `div` in the body: an insertion builds its parent's box again with
/// every box under it, so the second card goes in a parent of its own.
fn empty_div(doc: &Rc<RefCell<Document>>) -> NodeId {
    let mut doc = doc.borrow_mut();
    let body = doc.body();
    let div = doc.create_element("div");
    doc.insert_before(body, div, None);
    div
}

#[test]
fn a_second_mount_of_a_mounted_type_builds_no_box_outside_its_parent() {
    // Given
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let (first, second) = (empty_div(&doc), empty_div(&doc));
    let _card = StyledCard::mount(&doc, first, StyledCardProps::default());
    doc.borrow_mut().update(VIEWPORT).expect("updates");

    // When
    let _second = StyledCard::mount(&doc, second, StyledCardProps::default());
    let counts = update_counts(&doc);

    // Then: its parent, the new section, h2, p and their 2 text runs
    assert_eq!(counts.boxes_built, 6);
}

#[test]
fn a_mount_of_a_type_with_a_new_sheet_restyles_every_element() {
    // Given: only the title's sheet added so far
    let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
    let body = doc.borrow().body();
    let _title = StyledTitle::mount(&doc, body, StyledTitleProps::default());
    doc.borrow_mut().update(VIEWPORT).expect("updates");

    // When: the card adds its own sheet
    let _card = StyledCard::mount(&doc, body, StyledCardProps::default());
    let counts = update_counts(&doc);

    // Then: html, body, the first p and the card's 3 elements
    assert_eq!(counts.elements_restyled, 6);
}
