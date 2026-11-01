//! The text tests' page: a 160 x 24 px text field at the top left
//! holding "Warm" in 14 px Geist, a second empty one below it, a
//! disabled one below that, and a log of `input`, `change` and
//! `selectionchange` with the value.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

use super::{document, viewport};

const SHEET: &str = "body { margin: 0; font-size: 14px; } input { display: block; box-sizing: border-box; width: 160px; height: 24px; margin: 0; padding: 1px 2px; }";

pub type Log = Rc<RefCell<Vec<String>>>;

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    /// y 0..24, "Warm".
    pub name: NodeId,
    /// y 24..48, empty.
    pub plain: NodeId,
    /// y 48..72, disabled, "off".
    pub off: NodeId,
    pub log: Log,
}

pub fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let name = field(&mut doc, body, &[("value", "Warm")]);
    let plain = field(&mut doc, body, &[]);
    let off = field(&mut doc, body, &[("disabled", ""), ("value", "off")]);
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(400.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    let log = Log::default();
    for kind in ["input", "change", "selectionchange"] {
        let into = Rc::clone(&log);
        doc.borrow_mut().listen(
            name,
            kind,
            Box::new(move |e: &mut Event| {
                into.borrow_mut()
                    .push(format!("{} {:?}", e.name(), e.value()));
            }),
        );
    }
    Page {
        doc,
        name,
        plain,
        off,
        log,
    }
}

fn field(doc: &mut Document, parent: NodeId, attributes: &[(&str, &str)]) -> NodeId {
    let node = doc.create_element("input");
    doc.set_attribute(node, "type", "text");
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    doc.append(parent, node);
    node
}

pub fn value(page: &Page, node: NodeId) -> String {
    page.doc.borrow().value(node).expect("a field has a value")
}

pub fn selection(page: &Page, node: NodeId) -> (usize, usize) {
    page.doc
        .borrow()
        .selection(node)
        .expect("a field has a selection")
}

/// A key with the modifiers given, pressed and released.
pub fn key_with(page: &Page, key: &str, modifiers: vapors_elements::Modifiers) {
    Document::key_pressed(&page.doc, key, modifiers);
    Document::key_released(&page.doc, key, modifiers);
}

pub fn key(page: &Page, key: &str) {
    key_with(page, key, vapors_elements::Modifiers::default());
}

pub fn shift(key: &str) -> (&str, vapors_elements::Modifiers) {
    (
        key,
        vapors_elements::Modifiers {
            shift: true,
            ..vapors_elements::Modifiers::default()
        },
    )
}

pub fn control() -> vapors_elements::Modifiers {
    vapors_elements::Modifiers {
        control: true,
        ..vapors_elements::Modifiers::default()
    }
}

/// `text` typed, one character at a time as the platform sends it.
pub fn type_text(page: &Page, text: &str) {
    for c in text.chars() {
        let k = c.to_string();
        key(page, &k);
        Document::character_typed(&page.doc, &k);
    }
}

pub fn click(page: &Page, point: [f32; 2]) {
    Document::pointer_pressed(&page.doc, point);
    Document::pointer_released(&page.doc, point);
}

/// What Chromium's queued task does after an input event.
pub fn settle(page: &Page) {
    Document::settle_selection(&page.doc);
}
