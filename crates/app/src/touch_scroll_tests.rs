use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

use super::*;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 200.0,
    scale: 1.0,
};

/// `.outer` (100 px, 200 px of content) holding `.inner` (60 px tall,
/// 100 px of content) at its top.
fn nested() -> (Rc<RefCell<Document>>, NodeId, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let element = |doc: &mut Document, parent: NodeId, class: &str| {
        let node = doc.create_element("div");
        doc.set_attribute(node, "class", class);
        doc.append(parent, node);
        node
    };
    let outer = element(&mut doc, body, "outer");
    let inner = element(&mut doc, outer, "inner");
    element(&mut doc, inner, "filler");
    element(&mut doc, outer, "tail");
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } .outer { height: 100px; overflow: auto; } \
         .inner { height: 60px; overflow: auto; } .filler { height: 100px; } .tail { height: 140px; }",
    )
    .expect("parses");
    doc.update(VIEWPORT).expect("updates");
    (Rc::new(RefCell::new(doc)), outer, inner)
}

fn offset(doc: &Rc<RefCell<Document>>, node: NodeId) -> f32 {
    doc.borrow().scroll_offset(node).expect("scrolls")[1]
}

#[test]
fn a_gesture_keeps_the_container_it_first_moved() {
    // Given: a finger down on the inner list
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down([10.0, 10.0]);

    // When: it pans to the inner list's end, then past it
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);

    // Then: the inner list stops at its end; the outer one never moves
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 0.0);
}

#[test]
fn the_next_gesture_finds_its_container_again() {
    // Given: a gesture that left the inner list at its end
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();
    touch.down([10.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 50.0]);
    touch.up();

    // When: another gesture at the same place
    touch.down([10.0, 10.0]);
    touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);

    // Then: the outer one takes it
    assert_eq!(offset(&doc, inner), 40.0);
    assert_eq!(offset(&doc, outer), 30.0);
}

#[test]
fn a_pan_without_a_finger_down_scrolls_nothing() {
    // Given
    let (doc, outer, inner) = nested();
    let mut touch = TouchScroll::default();

    // When
    let moved = touch.pan(&mut doc.borrow_mut(), [0.0, 30.0]);

    // Then
    assert!(!moved);
    assert_eq!((offset(&doc, inner), offset(&doc, outer)), (0.0, 0.0));
}
