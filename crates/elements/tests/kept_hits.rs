//! Hits kept across updates (IU3 3c-3g): an update that paints slots
//! again patches their hits in painting order, and one that walks paint
//! order again gathers them afresh, so a hit test after any update finds
//! what a fresh paint's would. A debug build also compares the kept hits
//! with a fresh gather on every update (`kept_paint_check.rs`).

mod support;

use support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Painted};

/// Two 50px squares at the top left: `over` at 0 with z-index 2, `under`
/// at 20px with z-index 1, so `under` paints first; hit tested once, so
/// the hits are kept for the next update.
fn overlapping() -> (Document, NodeId, NodeId) {
    let mut doc = document();
    let css = "body { margin: 0; } \
        div { position: absolute; top: 0; width: 50px; height: 50px; } \
        .over { left: 0; z-index: 2; } .under { left: 20px; z-index: 1; } \
        .wide { width: 120px; background-color: red; } .top { z-index: 3; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let body = doc.body();
    let over = element(&mut doc, body, "div", "over");
    let under = element(&mut doc, body, "div", "under");
    let painted = doc.update(viewport(200.0, 200.0)).expect("updates");
    assert_eq!(painted.hit([30.0, 10.0]), Some(over));
    (doc, over, under)
}

fn updated(doc: &mut Document) -> &Painted {
    doc.update(viewport(200.0, 200.0)).expect("updates")
}

#[test]
fn a_repainted_lower_box_stays_under_and_hits_at_its_new_size() {
    // Given
    let (mut doc, over, under) = overlapping();

    // When: the lower box is painted again, wider, in the same place in paint order
    doc.set_attribute(under, "class", "under wide");
    let painted = updated(&mut doc);

    // Then
    assert_eq!(painted.hit([30.0, 10.0]), Some(over));
    assert_eq!(painted.hit([100.0, 10.0]), Some(under));
}

#[test]
fn a_removed_upper_box_leaves_the_lower_one_hit() {
    // Given
    let (mut doc, over, under) = overlapping();

    // When
    doc.remove(over);
    let painted = updated(&mut doc);

    // Then
    assert_eq!(painted.hit([30.0, 10.0]), Some(under));
    assert_eq!(painted.hit([10.0, 10.0]), None);
}

#[test]
fn a_lower_box_given_a_higher_z_index_takes_the_hit() {
    // Given
    let (mut doc, _, under) = overlapping();

    // When: a shape change; paint order is walked again
    doc.set_attribute(under, "class", "under top");
    let painted = updated(&mut doc);

    // Then
    assert_eq!(painted.hit([30.0, 10.0]), Some(under));
}
