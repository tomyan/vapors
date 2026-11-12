//! GPU tier: the `accessibility` phase of an `ElementsApp` frame takes
//! the a11y mirror's update when a mirror is attached (R6,
//! `docs/plans/r6.md`): every node on the first frame, what changed
//! after, nothing when nothing did; nothing at all with no mirror. A
//! refused take stops the frame naming the phase, and an update the
//! consumer did not take before the next frame is refused naming the
//! fix.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{app_frame, document};
use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxUpdate, Document};
use vapors_gpu_harness::gpu;
use vapors_platform::Next;

/// Buttons as flex items: the only layout vapors gives a `<button>`.
const CSS: &str = ".page { display: flex; font-size: 4px; } \
    button { width: 8px; height: 4px; margin: 0; padding: 0; }";

/// A page with two buttons, named by their text, and the text of the first.
fn buttons() -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = document(CSS, &[(None, "div", "page")]);
    let page = doc.children(doc.body())[0];
    let first = button(&mut doc, page, "One");
    button(&mut doc, page, "Two");
    (Rc::new(RefCell::new(doc)), first)
}

/// A button under `page` named `name`; its text node.
fn button(doc: &mut Document, page: NodeId, name: &str) -> NodeId {
    let button = doc.create_element("button");
    doc.append(page, button);
    let text = doc.create_text(name);
    doc.append(button, text);
    text
}

fn mirrored_app(doc: &Rc<RefCell<Document>>) -> ElementsApp {
    let mut app = ElementsApp::new(Rc::clone(doc));
    app.mirror_a11y(true);
    app
}

fn names(update: &AxUpdate) -> Vec<&str> {
    update.nodes.iter().map(|node| node.name.as_str()).collect()
}

#[test]
fn gpu_the_first_frame_takes_every_node_and_an_idle_frame_nothing() {
    // Given
    let (doc, _) = buttons();
    let mut app = mirrored_app(&doc);
    let gpu = gpu();

    // When
    app_frame(&mut app, &gpu);
    let first = app.ax_update().expect("the first frame's take");
    app_frame(&mut app, &gpu);
    let idle = app.ax_update();

    // Then
    assert!(first.every, "{first:?}");
    assert_eq!(names(&first), ["One", "Two"]);
    assert_eq!(idle, None);
}

#[test]
fn gpu_a_change_between_frames_is_the_next_frames_update() {
    // Given: a frame taken, then the first button renamed
    let (doc, text) = buttons();
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    app_frame(&mut app, &gpu);
    app.ax_update().expect("the first take");
    doc.borrow_mut().set_text(text, "Uno");

    // When
    app_frame(&mut app, &gpu);
    let update = app.ax_update().expect("the change");

    // Then
    assert!(!update.every, "{update:?}");
    assert_eq!(names(&update), ["Uno"]);
}

#[test]
fn gpu_with_no_mirror_attached_no_update_is_taken() {
    // Given
    let (doc, _) = buttons();
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let gpu = gpu();

    // When
    app_frame(&mut app, &gpu);

    // Then: the document's first take is still ahead
    assert_eq!(app.ax_update(), None);
    let take = doc.borrow_mut().take_ax_update().expect("takes");
    assert!(take.is_some_and(|take| take.every));
}

#[test]
fn gpu_a_refused_take_stops_the_frame_naming_the_phase() {
    // Given: a heading, which needs an aria-level the tree does not carry
    let (doc, _) = buttons();
    {
        let mut doc = doc.borrow_mut();
        let heading = doc.create_element("div");
        doc.set_attribute(heading, "role", "heading");
        doc.set_attribute(heading, "aria-label", "Levels");
        let page = doc.children(doc.body())[0];
        doc.append(page, heading);
    }
    let mut app = mirrored_app(&doc);
    let gpu = gpu();

    // When
    let (next, _) = app_frame(&mut app, &gpu);

    // Then
    assert_eq!(next, Next::Exit);
    let failure = app.failure().get().expect("the frame failed");
    assert!(
        failure.contains("Accessibility") && failure.contains("heading"),
        "{failure}"
    );
}

#[test]
fn gpu_an_update_not_taken_before_the_next_frame_is_refused_naming_the_fix() {
    // Given: a frame whose update nobody took
    let (doc, _) = buttons();
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    app_frame(&mut app, &gpu);

    // When
    let (next, _) = app_frame(&mut app, &gpu);

    // Then
    assert_eq!(next, Next::Exit);
    let failure = app.failure().get().expect("the frame failed");
    assert!(
        failure.contains("Accessibility") && failure.contains("ElementsApp::ax_update"),
        "{failure}"
    );
}

#[test]
fn gpu_a_mirror_attached_again_takes_every_node_again() {
    // Given: a mirror that took a frame, went, and a change meanwhile
    let (doc, text) = buttons();
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    app_frame(&mut app, &gpu);
    app.ax_update().expect("the first take");
    app.mirror_a11y(false);
    doc.borrow_mut().set_text(text, "Uno");
    app_frame(&mut app, &gpu);
    assert_eq!(app.ax_update(), None);

    // When
    app.mirror_a11y(true);
    app_frame(&mut app, &gpu);
    let update = app.ax_update().expect("the new mirror's first take");

    // Then
    assert!(update.every, "{update:?}");
    assert_eq!(names(&update), ["Uno", "Two"]);
}
