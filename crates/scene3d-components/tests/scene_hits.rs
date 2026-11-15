//! `<Scene>` answers the hits on its surface (docs/plans/css-for-3d.md
//! C4): the mesh under the pointer, picked through the camera, is the
//! target, its anchor's `pointer-events` and `visibility` honoured; a
//! moved node under a resting pointer moves the hover at the next
//! update.

mod support;

use support::{Rig, at};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};

const VIEWPORT: Viewport = Viewport {
    width: 64.0,
    height: 64.0,
    scale: 1.0,
};

/// The surface's centre: over the left mesh, 1 unit wide at the origin
/// (pixels 24 to 40 at 16 pixels a unit).
const CENTRE: [f32; 2] = [32.0, 32.0];
/// Over the right mesh at x 1, when shown (pixels 40 to 56).
const RIGHT: [f32; 2] = [48.0, 32.0];
/// Over neither.
const CORNER: [f32; 2] = [4.0, 4.0];

impl Rig {
    fn updated(&self) {
        self.doc.borrow_mut().update(VIEWPORT).expect("no refusal");
    }

    fn hit(&self, point: [f32; 2]) -> Option<NodeId> {
        self.doc.borrow().hit(point)
    }

    /// The tag and class of what `point` hits.
    fn hit_names(&self, point: [f32; 2]) -> (String, String) {
        let node = self.hit(point).expect("a hit");
        let doc = self.doc.borrow();
        let tag = doc.tag(node).unwrap_or_default().to_owned();
        let class = doc
            .current_attribute(node, "class")
            .unwrap_or_default()
            .to_owned();
        (tag, class)
    }
}

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
fn a_mesh_with_pointer_events_none_or_hidden_passes_the_hit_to_the_surface() {
    for css in [
        ".left { pointer-events: none; }",
        ".left { visibility: hidden; }",
    ] {
        // Given
        let rig = Rig::new();
        rig.doc
            .borrow_mut()
            .add_stylesheet("test.css", css)
            .unwrap();
        rig.updated();

        // Then: and the scene's answer is not refused
        assert_eq!(rig.hit_names(CENTRE).0, "gpu-surface", "{css}");
        rig.updated();
    }
}

#[test]
fn a_mesh_moved_from_under_a_resting_pointer_loses_the_hover() {
    // Given: the pointer rests on the left mesh, which shows its hover
    let rig = Rig::new();
    rig.doc
        .borrow_mut()
        .add_stylesheet("test.css", ".left:hover { color: #00ff00; }")
        .unwrap();
    rig.updated();
    Document::pointer_moved(&rig.doc, CENTRE);
    rig.updated();
    let left = rig.hit(CENTRE).expect("the left mesh");
    let hovered = rig.doc.borrow().scene_used(left).unwrap().colour.to_rgba8();

    // When: it moves away; nothing in the document changes
    rig.signals.left.set(at(-3.0));
    rig.updated();
    rig.updated();

    // Then
    let after = rig.doc.borrow().scene_used(left).unwrap().colour.to_rgba8();
    assert_eq!(hovered, [0, 255, 0, 255]);
    assert_ne!(after, [0, 255, 0, 255]);
}

#[test]
fn a_click_on_a_mesh_reaches_a_listener_on_its_anchor() {
    // Given
    let rig = Rig::new();
    rig.updated();
    let left = rig.hit(CENTRE).expect("the left mesh");
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = std::rc::Rc::clone(&clicks);
    let count = move |_: &mut vapors_dom::Event| counted.set(counted.get() + 1);
    rig.doc.borrow_mut().listen(left, "click", Box::new(count));

    // When
    Document::tapped(&rig.doc, CENTRE);

    // Then
    assert_eq!(clicks.get(), 1);
}

#[test]
fn a_still_pointer_over_a_still_scene_tests_no_hover() {
    // Given: the pointer rests on the left mesh, shown
    let rig = Rig::new();
    rig.updated();
    Document::pointer_moved(&rig.doc, CENTRE);
    rig.updated();
    rig.updated();

    // When
    rig.updated();

    // Then
    assert_eq!(rig.doc.borrow().update_counts().hovers_tested, 0);
}
