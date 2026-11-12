//! A checked checkbox's mark is placed from its content box by a
//! fraction of its size, which a shift does not check, so a moved
//! checked checkbox is painted again; an unchecked one shifts
//! (`kept_shift.rs`, IU3 3c-3e).

mod support;

use support::{document, element, viewport};
use vapors_dom::Dom;
use vapors_elements::Document;

/// A column with a spacer above one checkbox; moving the spacer's
/// height moves the checkbox by whole pixels.
fn column(checked: bool) -> (Document, vapors_dom::NodeId) {
    let mut doc = document();
    let body = doc.body();
    let spacer = element(&mut doc, body, "div", "");
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "checkbox");
    if checked {
        doc.set_attribute(input, "checked", "");
    }
    doc.set_attribute(spacer, "style", "height: 10px;");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; display: flex; flex-direction: column; height: 100px; }",
    )
    .expect("parses");
    (doc, spacer)
}

fn counts_after_moving(checked: bool) -> (usize, usize) {
    let (mut doc, spacer) = column(checked);
    doc.update(viewport(100.0, 100.0)).expect("updates");
    doc.set_attribute(spacer, "style", "height: 38px;");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    let counts = doc.update_counts();
    (counts.boxes_painted, counts.boxes_shifted)
}

#[test]
fn a_moved_checked_checkbox_is_painted_again() {
    // When
    let (painted, shifted) = counts_after_moving(true);

    // Then: the spacer and the checkbox
    assert_eq!((painted, shifted), (2, 0));
}

#[test]
fn a_moved_unchecked_checkbox_is_shifted() {
    // When
    let (painted, shifted) = counts_after_moving(false);

    // Then: the spacer painted, the checkbox shifted
    assert_eq!((painted, shifted), (1, 1));
}
