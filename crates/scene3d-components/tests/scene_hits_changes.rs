//! The hover under a resting pointer follows the scene
//! (docs/plans/css-for-3d.md C4): a mesh moved or removed from under it
//! loses it at the next update; a still scene tests nothing.

mod support;

use support::hits::{CENTRE, RIGHT};
use support::{Rig, at};
use vapors_dom::NodeId;
use vapors_elements::Document;

const GREEN: [u8; 4] = [0, 255, 0, 255];

/// The rig with the sheet `add` adds (a compiled one, tests/pages: a
/// sheet added from Rust beside the analysed page is refused, R1b), the
/// pointer resting at `point`, shown.
fn resting(add: fn(&Rig), point: [f32; 2]) -> Rig {
    let rig = Rig::new();
    add(&rig);
    rig.signals.shown.set(true);
    rig.updated();
    Document::pointer_moved(&rig.doc, point);
    rig.updated();
    rig
}

fn colour(rig: &Rig, anchor: NodeId) -> [u8; 4] {
    rig.doc
        .borrow()
        .scene_used(anchor)
        .unwrap()
        .colour
        .to_rgba8()
}

#[test]
fn a_mesh_moved_from_under_a_resting_pointer_loses_the_hover() {
    // Given: the left mesh shows its hover
    let rig = resting(
        |rig| crate::add_sheet!(rig.doc, left_hover::LeftHover),
        CENTRE,
    );
    let left = rig.hit(CENTRE).expect("the left mesh");
    let hovered = colour(&rig, left);

    // When: it moves away; nothing in the document changes
    rig.signals.left.set(at(-3.0));
    rig.updated();
    rig.updated();

    // Then
    assert_eq!(hovered, GREEN);
    assert_ne!(colour(&rig, left), GREEN);
}

#[test]
fn a_mesh_removed_from_under_a_resting_pointer_takes_the_hover_with_it() {
    // Given: the pointer on the right mesh; the group's hover shows on
    // the left mesh, its sibling
    let rig = resting(
        |rig| crate::add_sheet!(rig.doc, group_hover_left::GroupHoverLeft),
        RIGHT,
    );
    let left = rig.hit(CENTRE).expect("the left mesh");
    let hovered = colour(&rig, left);

    // When: the right mesh's {if} drops it
    rig.signals.shown.set(false);
    rig.updated();
    rig.updated();

    // Then: nothing of the group is under the pointer now
    assert_eq!(hovered, GREEN);
    assert_ne!(colour(&rig, left), GREEN);
}

#[test]
fn a_still_pointer_over_a_still_scene_tests_no_hover() {
    // Given: shown, and an update after
    let rig = resting(|_| {}, CENTRE);
    rig.updated();

    // When
    rig.updated();

    // Then
    assert_eq!(rig.doc.borrow().update_counts().hovers_tested, 0);
}
