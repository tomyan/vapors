//! A component's own events in the elements document: a dispatch from
//! inside a `click` listener reaches the node and its ancestors and
//! answers whether it was cancelled, as `dispatchEvent` from a click
//! handler does in a browser.

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use support::{document, element};
use vapors_dom::{Dispatcher, Dom, DynDom, Event};
use vapors_elements::Document;

#[test]
fn a_listener_dispatches_its_own_event_and_reads_its_cancel() {
    // Given: a button in a box; the box cancels `optionchoose`
    let mut doc = document();
    doc.add_stylesheet(
        "app.css",
        "body { margin: 0; } div { display: flex; } button { width: 40px; height: 20px; }",
    )
    .expect("parses");
    let body = doc.body();
    let outer = element(&mut doc, body, "div", "");
    let button = element(&mut doc, outer, "button", "");
    doc.listen(
        outer,
        "optionchoose",
        Box::new(|e: &mut Event| e.prevent_default()),
    );
    let doc = Rc::new(RefCell::new(doc));
    let events = Dispatcher::new(doc.dyn_dom(), &["optionchoose"]);
    let answer = Rc::new(RefCell::new(None));
    let into = Rc::clone(&answer);
    doc.borrow_mut().listen(
        button,
        "click",
        Box::new(move |_: &mut Event| {
            *into.borrow_mut() = Some(events.dispatch(button, Event::new("optionchoose")));
        }),
    );
    doc.borrow_mut()
        .update(support::viewport(200.0, 100.0))
        .expect("updates");

    // When
    Document::tapped(&doc, [5.0, 5.0]);

    // Then: it ran inside the click, and the box cancelled it
    assert_eq!(*answer.borrow(), Some(Ok(false)));
}
