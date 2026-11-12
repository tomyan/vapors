//! Shared by the `gpu_a11y_phase*` tests: a page of buttons named by
//! their text in a `SIZE`-square viewport, and an app with the a11y
//! mirror attached.

#![allow(dead_code)] // each test crate including this uses a part of it

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxUpdate, Document};

use super::document;

/// Buttons as flex items: the only layout vapors gives a `<button>`.
pub const CSS: &str = ".page { display: flex; font-size: 4px; } \
    button { width: 8px; height: 4px; margin: 0; padding: 0; }";

/// A page of `css`, holding the buttons `names` in a row (or a column,
/// as `css` lays `.page` out); each button's element.
pub fn buttons_page(css: &str, names: &[&str]) -> (Rc<RefCell<Document>>, Vec<NodeId>) {
    let mut doc = document(css, &[(None, "div", "page")]);
    let page = doc.children(doc.body())[0];
    let buttons = names.iter().map(|name| button(&mut doc, page, name));
    let buttons = buttons.collect();
    (Rc::new(RefCell::new(doc)), buttons)
}

/// A page with two buttons, named by their text, and the text of the first.
pub fn buttons() -> (Rc<RefCell<Document>>, NodeId) {
    let (doc, buttons) = buttons_page(CSS, &["One", "Two"]);
    let text = doc.borrow().children(buttons[0])[0];
    (doc, text)
}

/// A button under `page` named `name`; its element.
fn button(doc: &mut Document, page: NodeId, name: &str) -> NodeId {
    let button = doc.create_element("button");
    doc.append(page, button);
    let text = doc.create_text(name);
    doc.append(button, text);
    button
}

pub fn mirrored_app(doc: &Rc<RefCell<Document>>) -> ElementsApp {
    let mut app = ElementsApp::new(Rc::clone(doc));
    app.mirror_a11y(true);
    app
}

pub fn names(update: &AxUpdate) -> Vec<&str> {
    update.nodes.iter().map(|node| node.name.as_str()).collect()
}
