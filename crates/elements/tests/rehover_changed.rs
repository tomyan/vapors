//! After an update, the pointer's position is hit tested again only
//! when a hit this update removed or added holds it, or the hits were
//! gathered afresh (M11); otherwise the hover stands as it was. While
//! an element holds the mouse capture, the hover stays on it.

mod support;

use support::pointer::{quad_colours, setup, update};
use vapors_dom::{Dom, Event};
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
    // Given: the pointer rests on .a (y 0..20) in a scrolling row; .b
    // is below it
    let sheet = "body { margin: 0; } .row { height: 20px; overflow: auto; } .a, .b { height: 20px; } \
        .a:hover { background-color: #00ff00; } .b:hover { background-color: #0000ff; }";
    let (doc, row, _, _) = setup(sheet);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    update(&doc);

    // When: the row scrolls .b under the pointer, walking no paint
    // order; the second update shows the moved hover
    assert!(doc.borrow_mut().scroll_node_by(row, [0.0, 20.0]));
    update(&doc);
    let counts = doc.borrow().update_counts();
    update(&doc);

    // Then
    assert_eq!((counts.order_walks, counts.hovers_tested), (0, 1));
    let colours = quad_colours(&doc);
    assert!(colours.contains(&[0, 0, 255, 255]), "{colours:?}");
    assert!(!colours.contains(&[0, 255, 0, 255]), "{colours:?}");
}

#[test]
fn an_update_that_keeps_the_paint_does_not_hit_test() {
    // Given: the pointer rests on .a, shown by an update
    let (doc, _, _, _) = setup(SHEET);
    Document::pointer_moved(&doc, [5.0, 5.0]);
    update(&doc);

    // When: an update with no change keeps the last paint
    update(&doc);

    // Then
    assert_eq!(doc.borrow().update_counts().hovers_tested, 0);
    assert_eq!(quad_colours(&doc), [[0, 255, 0, 255]]);
}

#[test]
fn updates_while_captured_keep_hover_on_the_capturing_element() {
    // Given: .a captures the pointer on press, and the pointer moves
    // over .b while it holds it
    let (doc, _, a, _) = setup(SHEET);
    doc.borrow_mut().listen(
        a,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
    Document::pointer_moved(&doc, [5.0, 5.0]);
    Document::pointer_pressed(&doc, [5.0, 5.0]);
    Document::pointer_moved(&doc, [5.0, 30.0]);

    // When: two updates, the first restyling .a
    update(&doc);
    update(&doc);

    // Then: .a is still hovered, .b is not
    let colours = quad_colours(&doc);
    assert!(colours.contains(&[0, 255, 0, 255]), "{colours:?}");
    assert!(!colours.contains(&[0, 0, 255, 255]), "{colours:?}");
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
