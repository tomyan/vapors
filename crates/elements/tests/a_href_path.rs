//! The link a click follows is found on its path before the listeners
//! run, as in Chromium (checked in headless Chromium 155, 2026-11-14):
//! a listener that removes what was clicked, or the link itself, does
//! not stop it; a checkbox inside a link takes the click and the link is
//! not followed; a button inside a link follows it.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, viewport};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

/// A block link to `?demo=counter` at y 0..20 holding `inner` (a span,
/// a button or a checkbox, 10 px), updated; the link and `inner`.
fn link_holding(inner: &str) -> (Rc<RefCell<Document>>, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let link = doc.create_element("a");
    doc.set_attribute(link, "href", "?demo=counter");
    let held = doc.create_element(if inner == "checkbox" { "input" } else { inner });
    if inner == "checkbox" {
        doc.set_attribute(held, "type", "checkbox");
    }
    doc.append(link, held);
    doc.append(body, link);
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } a { display: block; height: 20px; } \
         a > * { display: block; width: 10px; height: 10px; margin: 0; padding: 0; }",
    )
    .expect("parses");
    doc.update(viewport(200.0, 100.0)).expect("updates");
    (Rc::new(RefCell::new(doc)), link, held)
}

fn click(doc: &Rc<RefCell<Document>>) {
    Document::pointer_pressed(doc, [5.0, 5.0]);
    Document::pointer_released(doc, [5.0, 5.0]);
}

fn taken(doc: &Rc<RefCell<Document>>) -> Option<String> {
    doc.borrow_mut().take_navigation().map(|link| link.href)
}

/// A click listener on `on` that removes `removed`.
fn removing(doc: &Rc<RefCell<Document>>, on: NodeId, removed: NodeId) {
    let writer = Rc::downgrade(doc);
    doc.borrow_mut().listen(
        on,
        "click",
        Box::new(move |_: &mut Event| {
            let doc = writer.upgrade().expect("the document lives");
            doc.borrow_mut().remove(removed);
        }),
    );
}

#[test]
fn a_listener_removing_what_was_clicked_does_not_stop_the_link() {
    // Given
    let (doc, _, span) = link_holding("span");
    removing(&doc, span, span);

    // When
    click(&doc);

    // Then
    assert_eq!(taken(&doc).as_deref(), Some("?demo=counter"));
}

#[test]
fn a_listener_removing_the_link_does_not_stop_it() {
    // Given
    let (doc, link, span) = link_holding("span");
    removing(&doc, span, link);

    // When
    click(&doc);

    // Then
    assert_eq!(taken(&doc).as_deref(), Some("?demo=counter"));
}

#[test]
fn a_checkbox_inside_a_link_toggles_and_the_link_is_not_followed() {
    // Given
    let (doc, _, checkbox) = link_holding("checkbox");

    // When
    click(&doc);

    // Then
    assert_eq!(doc.borrow().checked(checkbox), Some(true));
    assert_eq!(taken(&doc), None);
}

#[test]
fn a_button_inside_a_link_follows_it() {
    // Given
    let (doc, _, _) = link_holding("button");

    // When
    click(&doc);

    // Then
    assert_eq!(taken(&doc).as_deref(), Some("?demo=counter"));
}
