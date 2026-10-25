//! The inputs a `.pixels` file makes before a screenshot, on a demo's
//! document: what the native example and the gpu tier both call.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;

use crate::pixels;

/// Clicks at `point` (CSS pixels) as a mouse does: moves there,
/// presses and releases. What a `.pixels` file's `click` lines do
/// before a screenshot; the pointer stays there, hovering.
#[allow(clippy::cast_precision_loss)] // screen coordinates are small
pub fn click(document: &Rc<RefCell<Document>>, point: [u32; 2]) {
    let point = [point[0] as f32, point[1] as f32];
    Document::pointer_moved(document, point);
    Document::pointer_pressed(document, point);
    Document::pointer_released(document, point);
}

/// Makes a `.pixels` input: a click, or a wheel scroll at its point
/// (the pointer moves there first, as a mouse's does).
#[allow(clippy::cast_precision_loss)] // screen coordinates and deltas are small
pub fn input(document: &Rc<RefCell<Document>>, input: &pixels::Input) {
    match *input {
        pixels::Input::Click(point) => click(document, point),
        pixels::Input::Wheel { at, delta } => {
            let point = [at[0] as f32, at[1] as f32];
            Document::pointer_moved(document, point);
            document
                .borrow_mut()
                .scroll_by(point, [delta[0] as f32, delta[1] as f32]);
        }
    }
}
