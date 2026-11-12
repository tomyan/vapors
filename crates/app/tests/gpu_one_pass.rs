//! GPU tier: an `ElementsApp` frame of shadow, quad, text and icon runs
//! and the tooltip over them, all on the frame's one view, records them
//! in one render pass (vapors #181: about 110 µs of GPU time per pass
//! on ANV, and 9017 passes in one encoder failed with Out of Memory).

#[path = "support/surface.rs"]
mod support;
#[path = "support/tooltip.rs"]
mod tooltip;

use std::cell::RefCell;
use std::rc::Rc;

use support::{app_frame, app_frame_at, document, update};
use tooltip::TOOLTIP;
use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_gpu_harness::gpu;
use vapors_platform::{Event, Next, Platform};

/// Three titled rows, each with a shadow, a border, a letter and an icon.
fn busy_page() -> Rc<RefCell<Document>> {
    let css = ".row { width: 8px; height: 4px; border: 1px solid #ff0000; \
        box-shadow: 1px 1px 2px #000000; font-size: 4px; display: flex; }";
    let mut doc = document(&format!("{TOOLTIP} {css}"), &[]);
    let body = doc.body();
    for label in ["a", "b", "c"] {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.set_attribute(row, "title", ".");
        let text = doc.create_text(label);
        doc.append(row, text);
        let icon = icon(&mut doc);
        doc.append(row, icon);
        doc.append(body, row);
    }
    update(&mut doc, 1.0);
    Rc::new(RefCell::new(doc))
}

/// A 4 x 4 `svg` of one stroked line.
fn icon(doc: &mut Document) -> NodeId {
    let svg = doc.create_element("svg");
    let size = [("viewBox", "0 0 4 4"), ("width", "4"), ("height", "4")];
    for (name, value) in size {
        doc.set_attribute(svg, name, value);
    }
    doc.set_attribute(svg, "aria-hidden", "true");
    let path = doc.create_element("path");
    for (name, value) in [
        ("d", "M0 2H4"),
        ("fill", "none"),
        ("stroke", "currentColor"),
    ] {
        doc.set_attribute(path, name, value);
    }
    doc.append(svg, path);
    svg
}

#[test]
fn gpu_a_frame_of_every_run_kind_and_a_tooltip_on_one_view_records_one_render_pass() {
    // Given: the frame after a pointer move waits for the tooltip
    let gpu = gpu();
    let mut app = ElementsApp::new(busy_page());
    let moved = Event::PointerMoved { x: 2.0, y: 2.0 };
    app.give(&moved, &Platform::detached()).expect("placed");
    let (Next::WaitUntil(deadline), _) = app_frame(&mut app, &gpu) else {
        panic!("the frame after the move waits for the tooltip's deadline");
    };

    // When
    app_frame_at(&mut app, &gpu, deadline);

    // Then
    let counts = app.frame_counts();
    let kinds = [counts.shadows, counts.quads, counts.texts, counts.icons];
    assert!(
        kinds.iter().all(|kind| kind.draw_calls > 0),
        "every kind draws: {counts:?}"
    );
    assert_eq!(app.render_passes(), 1);
}
