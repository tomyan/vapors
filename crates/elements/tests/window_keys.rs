//! `<vapors:window>`'s listeners on the real document (`Dom::listen_window`):
//! every key reaches them, after the focused element's own listeners,
//! unless one stopped it; none after the listener is removed.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::document;
use vapors_dom::{Dom, Event};
use vapors_elements::{Document, Modifiers};

type Log = Rc<RefCell<Vec<String>>>;

/// A document with a button and a window listener for `keydown` and
/// `keyup`, each logging "window <name> <key>".
fn page() -> (Rc<RefCell<Document>>, vapors_dom::NodeId, Log) {
    let doc = Rc::new(RefCell::new(document()));
    let button = {
        let mut d = doc.borrow_mut();
        let button = d.create_element("button");
        let body = d.body();
        d.append(body, button);
        d.add_stylesheet("t.css", "button { display: block; }")
            .expect("parses");
        d.update(support::viewport(100.0, 50.0)).expect("updates");
        button
    };
    let log = Log::default();
    for name in ["keydown", "keyup"] {
        let into = Rc::clone(&log);
        doc.borrow_mut().listen_window(
            name,
            Box::new(move |e: &mut Event| {
                into.borrow_mut()
                    .push(format!("window {} {}", e.name(), e.key()));
            }),
        );
    }
    (doc, button, log)
}

fn press(doc: &Rc<RefCell<Document>>, key: &str) {
    Document::key_pressed(doc, key, Modifiers::default());
}

#[test]
fn a_key_with_nothing_focused_reaches_the_window() {
    // Given
    let (doc, _, log) = page();

    // When
    press(&doc, "ArrowLeft");
    Document::key_released(&doc, "ArrowLeft", Modifiers::default());

    // Then
    assert_eq!(
        *log.borrow(),
        ["window keydown ArrowLeft", "window keyup ArrowLeft"]
    );
}

#[test]
fn a_focused_elements_listener_runs_before_the_window_s() {
    // Given
    let (doc, button, log) = page();
    let into = Rc::clone(&log);
    doc.borrow_mut().listen(
        button,
        "keydown",
        Box::new(move |_: &mut Event| into.borrow_mut().push("button".into())),
    );
    Document::focus(&doc, Some(button));

    // When
    press(&doc, "ArrowUp");

    // Then
    assert_eq!(*log.borrow(), ["button", "window keydown ArrowUp"]);
}

#[test]
fn a_stopped_key_does_not_reach_the_window() {
    // Given
    let (doc, button, log) = page();
    doc.borrow_mut()
        .listen(button, "keydown", Box::new(Event::stop_propagation));
    Document::focus(&doc, Some(button));

    // When
    press(&doc, "ArrowUp");

    // Then
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
}

#[test]
fn a_removed_window_listener_hears_nothing() {
    // Given
    let doc = Rc::new(RefCell::new(document()));
    let log = Log::default();
    let into = Rc::clone(&log);
    let id = doc.borrow_mut().listen_window(
        "keydown",
        Box::new(move |_: &mut Event| into.borrow_mut().push("heard".into())),
    );

    // When
    doc.borrow_mut().unlisten_window(id);
    press(&doc, "a");

    // Then
    assert!(log.borrow().is_empty());
}

#[test]
#[should_panic(expected = "takes [\"keydown\", \"keyup\"] only")]
fn a_window_listener_for_a_click_is_refused() {
    // Given
    let mut doc = document();

    // When
    doc.listen_window("click", Box::new(|_: &mut Event| {}));
}
