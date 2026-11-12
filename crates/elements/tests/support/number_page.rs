//! The number tests' page: a 100 x 24 px number field at the top left
//! (0..100 by 1, at 20, 14 px text), a scrubbable one below it, a
//! disabled one below that, and a log of `input` and `change` with
//! their values.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

use super::{document, viewport};

const SHEET: &str = "body { margin: 0; font-size: 14px; } input { display: block; box-sizing: border-box; width: 100px; height: 24px; margin: 0; }";

/// Each `input` and `change` of `amount`, as `name value number`.
pub type Log = Rc<RefCell<Vec<String>>>;

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    /// y 0..24, 0..100 by 1 at 20.
    pub amount: NodeId,
    /// y 24..48, with `data-scrub`, 0..100 at 50.
    pub scrub: NodeId,
    /// y 48..72, disabled.
    pub off: NodeId,
    pub log: Log,
}

pub fn page() -> Page {
    page_with(&[("min", "0"), ("max", "100"), ("step", "1"), ("value", "20")])
}

/// The page with `amount`'s attributes as given.
pub fn page_with(attributes: &[(&str, &str)]) -> Page {
    let mut doc = document();
    let body = doc.body();
    let amount = number(&mut doc, body, attributes);
    let scrub = number(
        &mut doc,
        body,
        &[
            ("min", "0"),
            ("max", "100"),
            ("value", "50"),
            ("data-scrub", ""),
        ],
    );
    let off = number(&mut doc, body, &[("disabled", ""), ("value", "5")]);
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(400.0, 100.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));
    let log = Log::default();
    for (node, label) in [(amount, "amount"), (scrub, "scrub")] {
        for name in ["input", "change"] {
            let into = Rc::clone(&log);
            doc.borrow_mut().listen(
                node,
                name,
                Box::new(move |e: &mut Event| {
                    let number = e
                        .value_as_number()
                        .map_or("none".to_owned(), |n| n.to_string());
                    into.borrow_mut().push(format!(
                        "{label} {} {:?} {number}",
                        e.name(),
                        e.value()
                    ));
                }),
            );
        }
    }
    Page {
        doc,
        amount,
        scrub,
        off,
        log,
    }
}

fn number(doc: &mut Document, parent: NodeId, attributes: &[(&str, &str)]) -> NodeId {
    let node = doc.create_element("input");
    doc.set_attribute(node, "type", "number");
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    doc.append(parent, node);
    node
}

/// The field's DOM value.
pub fn value(page: &Page, node: NodeId) -> String {
    page.doc.borrow().value(node).expect("a field has a value")
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
