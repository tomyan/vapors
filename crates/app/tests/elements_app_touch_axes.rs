//! Touch pans follow the axes that scroll: a list that scrolls down
//! only (`overflow-x: hidden; overflow-y: auto`) takes a pan up and
//! passes a pan sideways to the container around it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::{App, Event, Platform, TouchPhase};
use vapors_text::FontSystem;

/// `.outer` (200 x 100, scrolls both ways) holding `.list` (100 x 50,
/// `list_css`) of a 300 x 160 child, and a 500 x 400 filler.
fn page(list_css: &str) -> (Rc<RefCell<Document>>, NodeId, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = format!(
        "body {{ margin: 0; }} .outer {{ width: 200px; height: 100px; overflow: auto; }} \
         .list {{ width: 100px; height: 50px; {list_css} }} \
         .wide {{ width: 300px; height: 160px; }} .fill {{ width: 500px; height: 400px; }}"
    );
    doc.add_stylesheet("app.css", &css).expect("parses");
    let mut add = |parent: Option<NodeId>, class: &str| {
        let node = doc.create_element("div");
        doc.set_attribute(node, "class", class);
        let parent = parent.unwrap_or_else(|| doc.body());
        doc.append(parent, node);
        node
    };
    let outer = add(None, "outer");
    let list = add(Some(outer), "list");
    add(Some(list), "wide");
    add(Some(outer), "fill");
    doc.update(Viewport {
        width: 200.0,
        height: 200.0,
        scale: 1.0,
    })
    .expect("updates");
    (Rc::new(RefCell::new(doc)), outer, list)
}

/// A finger down at (10, 10), one pan of `(dx, dy)`, and the lift.
fn pan(app: &mut ElementsApp, dx: f32, dy: f32) {
    let platform = Platform::detached();
    let touch = Event::Touch {
        id: 1,
        phase: TouchPhase::Started,
        x: 10.0,
        y: 10.0,
    };
    app.event(touch, &platform);
    app.event(Event::Pan { dx, dy }, &platform);
    let end = Event::Touch {
        id: 1,
        phase: TouchPhase::Ended,
        x: 10.0,
        y: 10.0,
    };
    app.event(end, &platform);
}

#[test]
fn a_pan_up_scrolls_a_list_that_scrolls_down_only() {
    // Given
    let (doc, outer, list) = page("overflow-x: hidden; overflow-y: auto;");
    let mut app = ElementsApp::new(Rc::clone(&doc));

    // When
    pan(&mut app, 0.0, -30.0);

    // Then
    assert_eq!(doc.borrow().scroll_offset(list), Some([0.0, 30.0]));
    assert_eq!(doc.borrow().scroll_offset(outer), Some([0.0, 0.0]));
}

#[test]
fn a_pan_sideways_passes_out_of_a_list_that_scrolls_down_only() {
    // Given
    let (doc, outer, list) = page("overflow-x: hidden; overflow-y: auto;");
    let mut app = ElementsApp::new(Rc::clone(&doc));

    // When
    pan(&mut app, -40.0, 0.0);

    // Then
    assert_eq!(doc.borrow().scroll_offset(list), Some([0.0, 0.0]));
    assert_eq!(doc.borrow().scroll_offset(outer), Some([40.0, 0.0]));
}

#[test]
fn a_pan_sideways_scrolls_a_list_that_scrolls_across_only() {
    // Given
    let (doc, outer, list) = page("overflow-x: auto; overflow-y: hidden;");
    let mut app = ElementsApp::new(Rc::clone(&doc));

    // When
    pan(&mut app, -40.0, 0.0);
    pan(&mut app, 0.0, -30.0);

    // Then: across by the list, down by the container
    assert_eq!(doc.borrow().scroll_offset(list), Some([40.0, 0.0]));
    assert_eq!(doc.borrow().scroll_offset(outer), Some([0.0, 30.0]));
}
