//! The checkbox tests' page: a checkbox at the top left, a checked one
//! below it, a disabled one below that, each 13 px with no margin, and
//! a log of `input` and `change` with their `checked`.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

use super::{document, viewport};

const SHEET: &str = "body { margin: 0; } input { display: block; margin: 0; }";

pub type Log = Rc<RefCell<Vec<String>>>;

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    /// y 0..13.
    pub visible: NodeId,
    /// y 13..26, checked.
    pub hidden: NodeId,
    /// y 26..39, disabled.
    pub off: NodeId,
    pub log: Log,
}

pub fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let visible = checkbox(&mut doc, body, &[]);
    let hidden = checkbox(&mut doc, body, &[("checked", "")]);
    let off = checkbox(&mut doc, body, &[("disabled", "")]);
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(100.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    let log = Log::default();
    for (node, label) in [(visible, "visible"), (hidden, "hidden")] {
        for name in ["input", "change"] {
            let into = Rc::clone(&log);
            doc.borrow_mut().listen(
                node,
                name,
                Box::new(move |e: &mut Event| {
                    into.borrow_mut()
                        .push(format!("{label} {} {}", e.name(), e.checked()));
                }),
            );
        }
    }
    Page {
        doc,
        visible,
        hidden,
        off,
        log,
    }
}

fn checkbox(doc: &mut Document, parent: NodeId, attributes: &[(&str, &str)]) -> NodeId {
    let node = doc.create_element("input");
    doc.set_attribute(node, "type", "checkbox");
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    doc.append(parent, node);
    node
}

pub fn checked(page: &Page, node: NodeId) -> bool {
    page.doc.borrow().checked(node).expect("a checkbox")
}

pub fn click(page: &Page, point: [f32; 2]) {
    Document::pointer_pressed(&page.doc, point);
    Document::pointer_released(&page.doc, point);
}

pub fn key(page: &Page, key: &str) {
    let none = vapors_elements::Modifiers::default();
    Document::key_pressed(&page.doc, key, none);
    Document::key_released(&page.doc, key, none);
}
