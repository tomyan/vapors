//! After an update, the pointer's position is hit tested again only
//! when a hit this update removed or added holds it, or the hits were
//! gathered afresh (M11); otherwise the hover stands as it was.

mod support;

use support::pointer::{quad_colours, setup, update};
use vapors_dom::Dom;
use vapors_elements::Document;

const SHEET: &str = "body { margin: 0; } .a, .b { height: 20px; } \
    .a:hover { background-color: #00ff00; } .b:hover { background-color: #0000ff; } \
    .marked { background-color: #ff0000; } .gone { display: none; }";

#[test]
fn an_update_away_from_the_pointer_keeps_hover_without_a_hit_test() {
    // Given: the pointer rests on .a (y 0..20)
    let (doc, _, _, b) = setup(SHEET);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    update(&doc);

    // When: .b (y 20..40) changes colour, moving no box
    doc.borrow_mut().set_attribute(b, "class", "b marked");
    update(&doc);

    // Then: the hover was not tested again, and .a still shows it
    assert_eq!(doc.borrow().update_counts().hovers_tested, 0);
    assert_eq!(quad_colours(&doc), [[0, 255, 0, 255], [255, 0, 0, 255]]);
}

#[test]
fn an_update_under_the_pointer_hit_tests_it_and_moves_hover() {
    // Given: the pointer rests on .a (y 0..20); .b is below it
    let (doc, _, a, _) = setup(SHEET);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    update(&doc);

    // When: .a disappears, so .b moves under the pointer; the second
    // update shows the moved hover
    doc.borrow_mut().set_attribute(a, "class", "a gone");
    update(&doc);
    let tested = doc.borrow().update_counts().hovers_tested;
    update(&doc);

    // Then
    assert_eq!(tested, 1);
    assert_eq!(quad_colours(&doc), [[0, 0, 255, 255]]);
}

#[test]
fn a_shape_change_away_from_the_pointer_hit_tests_it() {
    // Given: the pointer rests on .a; hits are gathered afresh after a
    // shape change, so whether the hover changed cannot be known
    let (doc, row, _, _) = setup(SHEET);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    update(&doc);

    // When: a box is added after .b
    let added = doc.borrow_mut().create_element("div");
    doc.borrow_mut().set_attribute(added, "class", "b");
    doc.borrow_mut().append(row, added);
    update(&doc);

    // Then: the hover was tested, and stands on .a
    assert_eq!(doc.borrow().update_counts().hovers_tested, 1);
    assert_eq!(quad_colours(&doc), [[0, 255, 0, 255]]);
}
