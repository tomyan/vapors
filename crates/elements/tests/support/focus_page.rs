//! The focus tests' page: buttons and tabindex boxes, in and out of
//! the Tab order, and a Tab key.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Modifiers};

use super::{document, element, viewport};

const SHEET: &str = "body { margin: 0; } button, div { display: block; height: 20px; }";

/// body > button.a, div.b[tabindex=0], button.c[disabled],
/// div.d[tabindex=-1], div.hidden[tabindex=0] (display: none), button.e
pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    pub save: NodeId,
    pub note: NodeId,
    pub aside: NodeId,
    pub cancel: NodeId,
}

pub fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let save = element(&mut doc, body, "button", "a");
    let note = element(&mut doc, body, "div", "b");
    doc.set_attribute(note, "tabindex", "0");
    let gone = element(&mut doc, body, "button", "c");
    doc.set_attribute(gone, "disabled", "");
    let aside = element(&mut doc, body, "div", "d");
    doc.set_attribute(aside, "tabindex", "-1");
    let hidden = element(&mut doc, body, "div", "hidden");
    doc.set_attribute(hidden, "tabindex", "0");
    let cancel = element(&mut doc, body, "button", "e");
    doc.add_stylesheet("app.css", &format!("{SHEET} .hidden {{ display: none; }}"))
        .expect("parses");
    doc.update(viewport(100.0, 200.0)).expect("updates");
    Page {
        doc: Rc::new(RefCell::new(doc)),
        save,
        note,
        aside,
        cancel,
    }
}

pub fn tab(doc: &Rc<RefCell<Document>>, shift: bool) {
    let modifiers = Modifiers {
        shift,
        ..Modifiers::default()
    };
    Document::key_pressed(doc, "Tab", modifiers);
    Document::key_released(doc, "Tab", modifiers);
}
