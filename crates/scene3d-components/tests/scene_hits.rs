//! `<Scene>` answers the hits on its surface (docs/plans/css-for-3d.md
//! C4): the mesh under the pointer, picked through the camera, is the
//! target; a mesh whose anchor does not take the pointer is passed
//! over; a mesh the application added blocks it.

mod support;

use std::cell::Cell;
use std::rc::Rc;
use support::hits::{CENTRE, CORNER, RIGHT};
use support::{Rig, square};
use vapors_dom::{Dom, Event};
use vapors_elements::Document;
use vapors_scene3d::{Colour, NodeKind, Transform, Vec3};

#[test]
fn the_mesh_under_the_pointer_is_the_target() {
    // Given
    let rig = Rig::new();
    rig.signals.shown.set(true);
    rig.updated();

    // Then
    assert_eq!(rig.hit_names(CENTRE).0, "mesh");
    assert!(rig.hit_names(CENTRE).1.contains("left"));
    assert_eq!(rig.hit_names(RIGHT).0, "mesh");
    assert!(!rig.hit_names(RIGHT).1.contains("left"));
    assert_eq!(rig.hit_names(CORNER).0, "gpu-surface");
}

#[test]
fn a_mesh_whose_anchor_does_not_take_the_pointer_passes_the_hit_to_the_surface() {
    let cases = [
        ("pointer-events: none", ".left { pointer-events: none; }"),
        ("hidden", ".left { visibility: hidden; }"),
    ];
    for (case, css) in cases {
        // Given
        let rig = Rig::new();
        rig.doc
            .borrow_mut()
            .add_stylesheet("test.css", css)
            .unwrap();
        rig.updated();

        // When
        let hit = rig.hit_names(CENTRE).0;

        // Then: and the scene's answer is not refused
        assert_eq!(hit, "gpu-surface", "{case}");
        rig.updated();
    }
}

#[test]
fn a_mesh_the_application_added_in_front_blocks_the_pointer() {
    // Given: a square added straight to the scene, in front of the left
    // mesh, which no <Mesh> mounted
    let rig = Rig::new();
    let geometry = square(&rig.meshes);
    let kind = NodeKind::Mesh {
        geometry,
        colour: Colour::BLACK,
    };
    let front = Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)).unwrap();
    let root = rig.scene.borrow().root();
    rig.scene.borrow_mut().add(root, kind, front).unwrap();
    rig.updated();

    // When
    let hit = rig.hit_names(CENTRE).0;

    // Then: the surface, not the mesh drawn behind it
    assert_eq!(hit, "gpu-surface");
}

#[test]
fn a_click_on_a_mesh_reaches_a_listener_on_its_anchor() {
    // Given
    let rig = Rig::new();
    rig.updated();
    let left = rig.hit(CENTRE).expect("the left mesh");
    let clicks = Rc::new(Cell::new(0));
    let counted = Rc::clone(&clicks);
    let count = move |_: &mut Event| counted.set(counted.get() + 1);
    rig.doc.borrow_mut().listen(left, "click", Box::new(count));

    // When
    Document::tapped(&rig.doc, CENTRE);

    // Then
    assert_eq!(clicks.get(), 1);
}
