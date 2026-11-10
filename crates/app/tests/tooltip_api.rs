//! A host outside the crate names the tooltip's types: what
//! `DocumentInput::tooltip` returns, what brought it up and the delay.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{DocumentInput, TOOLTIP_DELAY, Tooltip, TooltipTrigger};
use vapors_dom::Dom;
use vapors_elements::{Document, Viewport};
use vapors_platform::{Event, Instant, Platform};
use vapors_text::FontSystem;

/// A 100x20 button titled "Open (o)" at the top left, updated.
fn page() -> Rc<RefCell<Document>> {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = "body { margin: 0; display: flex; } button { width: 100px; height: 20px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    let (body, open) = (doc.body(), doc.create_element("button"));
    doc.set_attribute(open, "title", "Open (o)");
    doc.append(body, open);
    let viewport = Viewport {
        width: 200.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    Rc::new(RefCell::new(doc))
}

#[test]
fn a_host_gets_the_pointers_tooltip_by_its_public_types() {
    // Given
    let doc = page();
    let mut input = DocumentInput::new();
    let before = Instant::now();

    // When
    let moved = Event::PointerMoved { x: 10.0, y: 10.0 };
    input
        .give(&doc, &moved, &Platform::detached())
        .expect("placed");
    let deadline = input.tooltip_deadline().expect("armed");
    let tip: Option<Tooltip> = input.tooltip(&doc.borrow(), deadline).cloned();

    // Then
    assert!(deadline >= before + TOOLTIP_DELAY);
    let tip = tip.expect("shown at the deadline");
    assert_eq!(
        (tip.text.as_str(), tip.trigger),
        ("Open (o)", TooltipTrigger::Pointer)
    );
}
