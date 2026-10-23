//! Shared by the pointer tests: a small laid-out tree, a listener that
//! logs, and an update at the tests' viewport.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

use super::{document, element, viewport};

pub type Log = Rc<RefCell<Vec<String>>>;

pub const SHEET: &str = "body { margin: 0; } .a { height: 20px; } .b { height: 20px; }";

/// body > .row (40 px) > .a (y 0..20), .b (y 20..40); updated once.
pub fn setup(sheet: &str) -> (Rc<RefCell<Document>>, NodeId, NodeId, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let a = element(&mut doc, row, "div", "a");
    let b = element(&mut doc, row, "div", "b");
    doc.add_stylesheet("app.css", sheet).expect("parses");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    (Rc::new(RefCell::new(doc)), row, a, b)
}

pub fn record(doc: &Rc<RefCell<Document>>, node: NodeId, event: &str, label: &str, log: &Log) {
    let (log, label) = (Rc::clone(log), label.to_owned());
    doc.borrow_mut().listen(
        node,
        event,
        Box::new(move |e: &mut Event| log.borrow_mut().push(format!("{label} {}", e.name()))),
    );
}

pub fn update(doc: &Rc<RefCell<Document>>) {
    doc.borrow_mut()
        .update(viewport(100.0, 100.0))
        .expect("updates");
}
