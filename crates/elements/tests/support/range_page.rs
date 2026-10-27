//! The range tests' page: a 200 x 20 px range at x 0..200 (0..100 by
//! 1, at 20), a disabled one below it, and a log of its `input` and
//! `change` events with their values.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

use super::{document, viewport};

const SHEET: &str = "body { margin: 0; } input { display: block; box-sizing: border-box; width: 200px; height: 20px; margin: 0; padding: 0; }";

/// Each `input` and `change` the range fired, as `name value number`.
pub type Log = Rc<RefCell<Vec<String>>>;

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    /// y 0..20, 0..100 by 1 at 20.
    pub level: NodeId,
    /// y 20..40, disabled (or live, from `page_with_second`).
    pub off: NodeId,
    pub log: Log,
}

/// The page, updated once, its `level` logging.
pub fn page() -> Page {
    page_with(&[("min", "0"), ("max", "100"), ("step", "1"), ("value", "20")])
}

/// The page with a second live range in place of the disabled one.
pub fn page_with_second() -> Page {
    build(
        &[("min", "0"), ("max", "100"), ("step", "1"), ("value", "20")],
        &[],
    )
}

/// The page with `level`'s attributes as given.
pub fn page_with(attributes: &[(&str, &str)]) -> Page {
    build(attributes, &[("disabled", "")])
}

fn build(attributes: &[(&str, &str)], second: &[(&str, &str)]) -> Page {
    let mut doc = document();
    let body = doc.body();
    let level = range(&mut doc, body, attributes);
    let off = range(&mut doc, body, second);
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(400.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    let log = Log::default();
    for name in ["input", "change"] {
        let into = Rc::clone(&log);
        doc.borrow_mut().listen(
            level,
            name,
            Box::new(move |e: &mut Event| {
                into.borrow_mut().push(format!(
                    "{} {} {}",
                    e.name(),
                    e.value(),
                    e.value_as_number()
                ));
            }),
        );
    }
    Page {
        doc,
        level,
        off,
        log,
    }
}

fn range(doc: &mut Document, parent: NodeId, attributes: &[(&str, &str)]) -> NodeId {
    let node = doc.create_element("input");
    doc.set_attribute(node, "type", "range");
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    doc.append(parent, node);
    node
}

/// The range's value as text.
pub fn value(page: &Page, node: NodeId) -> String {
    page.doc.borrow().value(node).expect("a range has a value")
}

/// A key pressed and released with no modifiers.
pub fn key(page: &Page, key: &str) {
    let none = vapors_elements::Modifiers::default();
    Document::key_pressed(&page.doc, key, none);
    Document::key_released(&page.doc, key, none);
}

/// A key pressed and released with Shift held.
pub fn shift_key(page: &Page, key: &str) {
    let shift = vapors_elements::Modifiers {
        shift: true,
        ..vapors_elements::Modifiers::default()
    };
    Document::key_pressed(&page.doc, key, shift);
    Document::key_released(&page.doc, key, shift);
}
