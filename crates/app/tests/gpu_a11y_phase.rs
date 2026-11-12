//! GPU tier: the `accessibility` phase of an `ElementsApp` frame takes
//! the a11y mirror's update when a mirror is attached (R6,
//! `docs/plans/r6.md`): every node on the first frame, what changed
//! after, nothing when nothing did; nothing at all with no mirror. A
//! refused take stops the frame naming the phase, and an update the
//! consumer did not take before the next frame is refused naming the
//! fix.

#[path = "support/surface.rs"]
mod support;

use std::rc::Rc;

use support::a11y_buttons::{buttons, mirrored_app, names};
use support::app_frame;
use vapors_app::ElementsApp;
use vapors_dom::Dom;
use vapors_gpu_harness::gpu;
use vapors_platform::Next;

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
    app.mirror_a11y(false).expect("detaches");
    doc.borrow_mut().set_text(text, "Uno");
    app_frame(&mut app, &gpu);
    assert_eq!(app.ax_update(), None);

    // When
    app.mirror_a11y(true).expect("attaches again");
    app_frame(&mut app, &gpu);
    let update = app.ax_update().expect("the new mirror's first take");

    // Then
    assert!(update.every, "{update:?}");
    assert_eq!(names(&update), ["Uno", "Two"]);
}

#[test]
fn gpu_a_take_outside_the_app_is_refused_naming_ax_update_while_attached() {
    // Given
    let (doc, _) = buttons();
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    app_frame(&mut app, &gpu);

    // When
    let refused = doc.borrow_mut().take_ax_update().expect_err("owned");

    // Then
    assert!(
        refused.to_string().contains("ElementsApp::ax_update"),
        "{refused}"
    );
}

#[test]
fn gpu_a_second_app_on_the_document_cannot_attach_a_mirror() {
    // Given
    let (doc, _) = buttons();
    let _first = mirrored_app(&doc);
    let mut second = ElementsApp::new(Rc::clone(&doc));

    // When
    let refused = second.mirror_a11y(true).expect_err("owned");

    // Then
    assert!(
        refused.to_string().contains("ElementsApp::ax_update"),
        "{refused}"
    );
}
