//! GPU tier: the frame's own steps after `Document::update` (the focus
//! fixup's blur, the rehover under a resting pointer) change the
//! document, and the Accessibility phase must still take an update
//! (review-v242 H1): the Update phase updates once more when they did,
//! so an app with a mirror attached does not exit on disabling the
//! focused button, removing the element under the pointer, or
//! scrolling under a resting pointer.

#[path = "support/surface.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::a11y_buttons::{CSS, buttons_page, mirrored_app, names};
use support::app_frame;
use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxUpdate, Document};
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::{Event, Next, Platform};

/// A column of buttons in a body that scrolls: 20 rows of 4px in a
/// 32px viewport.
const SCROLLING_CSS: &str = "html { overflow: hidden; } body { overflow: auto; height: 32px; } \
    .page { display: flex; flex-direction: column; font-size: 4px; } \
    button { width: 8px; height: 4px; margin: 0; padding: 0; }";

/// One frame that must not stop: its a11y update.
fn frame_taken(app: &mut ElementsApp, gpu: &Gpu) -> AxUpdate {
    let (next, _) = app_frame(app, gpu);
    assert_ne!(next, Next::Exit, "{:?}", app.failure().get());
    assert_eq!(app.failure().get(), None);
    app.ax_update().expect("the frame's take")
}

/// The pointer resting at `x, y` after a frame: the app has drawn, so
/// the move lands, and the frame after it restyles the hover.
fn pointer_resting(app: &mut ElementsApp, gpu: &Gpu, x: f32, y: f32) {
    let platform = Platform::detached();
    app_frame(app, gpu);
    app.ax_update();
    app.give(&Event::PointerMoved { x, y }, &platform)
        .expect("placed");
    app_frame(app, gpu);
    app.ax_update();
}

fn record<'a>(update: &'a AxUpdate, name: &str) -> &'a vapors_elements::AxMirrorNode {
    update
        .nodes
        .iter()
        .find(|node| node.name == name)
        .unwrap_or_else(|| panic!("no record for {name} in {update:?}"))
}

#[test]
fn gpu_disabling_the_focused_button_blurs_it_and_the_frame_goes_on() {
    // Given: the first button focused and taken
    let (doc, buttons) = buttons_page(CSS, &["One", "Two"]);
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    app_frame(&mut app, &gpu);
    app.ax_update();
    Document::focus(&doc, Some(buttons[0]));
    frame_taken(&mut app, &gpu);
    doc.borrow_mut().set_attribute(buttons[0], "disabled", "");

    // When: the fixup blurs it after the update
    let update = frame_taken(&mut app, &gpu);

    // Then: one frame holds the blur and the disabled record
    assert_eq!(update.focus, Some(None), "{update:?}");
    assert!(record(&update, "One").states.disabled, "{update:?}");
    assert!(!record(&update, "One").states.focused, "{update:?}");
}

#[test]
fn gpu_removing_the_element_under_the_pointer_rehovers_and_the_frame_goes_on() {
    // Given: the pointer resting over the second button
    let (doc, buttons) = buttons_page(CSS, &["One", "Two"]);
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    pointer_resting(&mut app, &gpu, 10.0, 2.0);
    doc.borrow_mut().remove(buttons[0]);

    // When: the second button moves under the pointer
    let update = frame_taken(&mut app, &gpu);

    // Then
    assert_eq!(update.removed, [buttons[0]], "{update:?}");
    assert_eq!(names(&update), ["Two"]);
    assert_eq!(record(&update, "Two").bounds.x, 0.0, "{update:?}");
}

#[test]
fn gpu_scrolling_under_a_resting_pointer_rehovers_and_the_frame_goes_on() {
    // Given: the pointer resting over the first row of a scrolling body
    let rows: Vec<String> = (1..=20).map(|row| format!("Row {row}")).collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (doc, _) = buttons_page(SCROLLING_CSS, &rows);
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    pointer_resting(&mut app, &gpu, 4.0, 2.0);
    scroll_body(&doc, 4.0);

    // When: the second row moves under the pointer
    let update = frame_taken(&mut app, &gpu);

    // Then: every row moved
    assert_eq!(update.nodes.len(), 20, "{update:?}");
    assert_eq!(record(&update, "Row 2").bounds.y, 0.0, "{update:?}");
}

fn scroll_body(doc: &Rc<RefCell<Document>>, by: f32) {
    let mut doc = doc.borrow_mut();
    let body: NodeId = doc.body();
    assert!(doc.scroll_node_by(body, [0.0, by]), "the body scrolls");
}

#[test]
fn gpu_a_hover_style_that_moves_what_is_under_the_pointer_stops_the_frame_named() {
    // Given: hovering a row moves it from under the pointer, and
    // leaving it moves it back
    let css = format!("{SCROLLING_CSS} button:hover {{ margin-top: 4px; }}");
    let (doc, _) = buttons_page(&css, &["Row 1", "Row 2", "Row 3"]);
    let mut app = mirrored_app(&doc);
    let gpu = gpu();
    app_frame(&mut app, &gpu);
    app.ax_update();
    app.give(
        &Event::PointerMoved { x: 4.0, y: 5.0 },
        &Platform::detached(),
    )
    .expect("placed");

    // When
    let (next, _) = app_frame(&mut app, &gpu);

    // Then
    assert_eq!(next, Next::Exit);
    let failure = app.failure().get().expect("the frame failed");
    assert!(
        failure.contains("Update") && failure.contains("two updates"),
        "{failure}"
    );
}
