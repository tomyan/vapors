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

#[test]
fn the_pointer_position_is_unknown_until_the_pointer_moves() {
    // Given
    let doc = page();
    let mut input = DocumentInput::new();
    let before = input.pointer();

    // When
    let moved = Event::PointerMoved { x: 12.0, y: 7.0 };
    input
        .give(&doc, &moved, &Platform::detached())
        .expect("placed");

    // Then
    assert_eq!((before, input.pointer()), (None, Some([12.0, 7.0])));
}

#[test]
fn the_pointer_position_stays_after_the_pointer_leaves_the_window() {
    // Given
    let doc = page();
    let mut input = DocumentInput::new();
    let moved = Event::PointerMoved { x: 12.0, y: 7.0 };
    input
        .give(&doc, &moved, &Platform::detached())
        .expect("placed");

    // When
    let _ = input.give(&doc, &Event::PointerLeft, &Platform::detached());

    // Then
    assert_eq!(input.pointer(), Some([12.0, 7.0]));
}

#[test]
fn the_hotspot_is_where_the_pointer_was_when_the_tooltip_showed() {
    // Given: shown at (10, 10)
    let doc = page();
    let mut input = DocumentInput::new();
    let platform = Platform::detached();
    let moved = Event::PointerMoved { x: 10.0, y: 10.0 };
    input.give(&doc, &moved, &platform).expect("placed");
    let deadline = input.tooltip_deadline().expect("armed");
    let _ = input.tooltip(&doc.borrow(), deadline);

    // When: a move within the button, and the next frame's ask
    let within = Event::PointerMoved { x: 15.0, y: 12.0 };
    input.give(&doc, &within, &platform).expect("placed");
    let shown = input.tooltip(&doc.borrow(), deadline).is_some();

    // Then
    assert!(shown, "a move within the title keeps the tooltip");
    assert_eq!(input.tooltip_hotspot(), Some([10.0, 10.0]));
}

#[test]
fn there_is_no_hotspot_once_the_tooltip_hides() {
    // Given: shown at (10, 10)
    let doc = page();
    let mut input = DocumentInput::new();
    let platform = Platform::detached();
    let moved = Event::PointerMoved { x: 10.0, y: 10.0 };
    input.give(&doc, &moved, &platform).expect("placed");
    let deadline = input.tooltip_deadline().expect("armed");
    let _ = input.tooltip(&doc.borrow(), deadline);

    // When
    let _ = input.give(&doc, &Event::PointerLeft, &platform);

    // Then
    assert_eq!(input.tooltip_hotspot(), None);
}
