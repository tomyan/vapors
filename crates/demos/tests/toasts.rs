//! The toasts demo (F2c S8) between frames, which its `.pixels` clicks
//! cannot reach: each click there is hit against the frame before the
//! clicks, so a toast one click raises cannot be dismissed by the next.
//! Here the document is updated after every click.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::{Document, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};
const SAVE: [u32; 2] = [530, 21];
/// The Dismiss button of the bottom toast, and a point on its message.
const BOTTOM_DISMISS: [u32; 2] = [600, 362];
const BOTTOM_TOAST: [f32; 2] = [380.0, 362.0];
/// A point on the toast above it.
const SECOND_TOAST: [f32; 2] = [380.0, 302.0];

fn shown() -> Rc<RefCell<Document>> {
    let demo = vapors_demos::find("toasts").expect("a demo");
    let document = vapors_demos::document(&demo).document;
    update(&document);
    document
}

fn update(document: &Rc<RefCell<Document>>) {
    if let Err(errors) = document.borrow_mut().update(VIEWPORT) {
        panic!("toasts was refused: {errors:#?}");
    }
}

fn click(document: &Rc<RefCell<Document>>, point: [u32; 2]) {
    vapors_demos::click(document, point);
    update(document);
}

/// The class of what is hit at `point`, or of its nearest element.
fn class_at(document: &Rc<RefCell<Document>>, point: [f32; 2]) -> String {
    let mut doc = document.borrow_mut();
    let node = doc
        .update(VIEWPORT)
        .expect("updates")
        .hit(point)
        .expect("a hit");
    doc.attribute(node, "class").unwrap_or_default().to_owned()
}

#[test]
fn a_toast_stays_over_the_canvas_until_dismissed() {
    // Given: two toasts raised
    let document = shown();
    click(&document, SAVE);
    click(&document, SAVE);
    assert_eq!(class_at(&document, SECOND_TOAST), "message");

    // When: the bottom one is dismissed
    click(&document, BOTTOM_DISMISS);

    // Then: the other moves down into its place and the slot above is canvas
    assert_eq!(class_at(&document, BOTTOM_TOAST), "message");
    assert_eq!(class_at(&document, SECOND_TOAST), "canvas");
}

#[test]
fn a_toast_is_hit_over_the_canvas() {
    // Given
    let document = shown();

    // When
    click(&document, SAVE);

    // Then: the fixed stack is above the canvas region
    assert_eq!(class_at(&document, BOTTOM_TOAST), "message");
}
