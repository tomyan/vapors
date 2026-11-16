//! The link tests' page: a link to `?demo=counter` holding a span at
//! y 0..20, and an `a` without `href` at y 20..40, both blocks.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Modifiers, Navigation};

use super::{document, viewport};

const SHEET: &str = "body { margin: 0; } a { display: block; height: 20px; }";

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    /// y 0..20, `href="?demo=counter"`, holding a span.
    pub link: NodeId,
    /// y 20..40, no `href`.
    pub plain: NodeId,
}

pub fn page() -> Page {
    let mut doc = document();
    let body = doc.body();
    let link = anchor(&mut doc, body, Some("?demo=counter"), "Counter");
    let plain = anchor(&mut doc, body, None, "Nowhere");
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(viewport(200.0, 100.0)).expect("updates");
    Page {
        doc: Rc::new(RefCell::new(doc)),
        link,
        plain,
    }
}

fn anchor(doc: &mut Document, parent: NodeId, href: Option<&str>, label: &str) -> NodeId {
    let a = doc.create_element("a");
    if let Some(href) = href {
        doc.set_attribute(a, "href", href);
    }
    let span = doc.create_element("span");
    let text = doc.create_text(label);
    doc.append(span, text);
    doc.append(a, span);
    doc.append(parent, a);
    a
}

pub fn click(page: &Page, point: [f32; 2]) {
    Document::pointer_pressed(&page.doc, point);
    Document::pointer_released(&page.doc, point);
}

pub fn key(page: &Page, key: &str) {
    let none = Modifiers::default();
    Document::key_pressed(&page.doc, key, none);
    Document::key_released(&page.doc, key, none);
}

/// The href of the link the page was asked to follow, once.
pub fn taken(page: &Page) -> Option<String> {
    followed(page).map(|link| link.href)
}

/// The link the page was asked to follow, once.
pub fn followed(page: &Page) -> Option<Navigation> {
    page.doc.borrow_mut().take_navigation()
}

/// A click at `point` with `modifiers` held.
pub fn click_with(page: &Page, point: [f32; 2], modifiers: Modifiers) {
    Document::pointer_pressed_with(&page.doc, point, NonZeroU32::MIN, modifiers);
    Document::pointer_released_with(&page.doc, point, modifiers);
}

/// `key` pressed and released with `modifiers` held.
pub fn key_with(page: &Page, key: &str, modifiers: Modifiers) {
    Document::key_pressed(&page.doc, key, modifiers);
    Document::key_released(&page.doc, key, modifiers);
}
