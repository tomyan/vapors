//! The listener-removal sweep's page: one of each control the tree
//! drives, each 100 x 24 px in a wrapper of its own, stacked from the
//! top: a button, a checkbox, a text field, a number field, a
//! scrubbable number field and a range.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{Document, Viewport};

use super::{document, viewport};

const SHEET: &str = "body { margin: 0; font-size: 14px; } div, input, button { display: block; box-sizing: border-box; width: 100px; height: 24px; margin: 0; padding: 0; }";

/// A control: its name, tag and attributes.
pub type Control = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

pub const CONTROLS: &[Control] = &[
    ("button", "button", &[]),
    ("checkbox", "input", &[("type", "checkbox")]),
    ("text", "input", &[("type", "text"), ("value", "Warm")]),
    ("number", "input", &[("type", "number"), ("value", "20")]),
    (
        "scrub",
        "input",
        &[("type", "number"), ("value", "50"), ("data-scrub", "")],
    ),
    ("range", "input", &[("type", "range"), ("value", "50")]),
];

/// What a listener on the control removes.
#[derive(Clone, Copy, Debug)]
pub enum Removed {
    Target,
    Parent,
}

pub struct Page {
    pub doc: Rc<RefCell<Document>>,
    /// The control under test.
    pub control: NodeId,
    /// A point in its middle.
    pub point: [f32; 2],
}

pub fn the_viewport() -> Viewport {
    viewport(400.0, 200.0)
}

/// The page, with the control at `index` in [`CONTROLS`] under test.
pub fn page(index: usize) -> Page {
    let mut doc = document();
    let body = doc.body();
    let controls: Vec<NodeId> = CONTROLS
        .iter()
        .map(|(_, tag, attributes)| wrapped(&mut doc, body, tag, attributes))
        .collect();
    doc.add_stylesheet("app.css", SHEET).expect("parses");
    doc.update(the_viewport()).expect("updates");
    #[allow(clippy::cast_precision_loss)] // six controls
    let y = 24.0 * index as f32 + 12.0;
    Page {
        doc: Rc::new(RefCell::new(doc)),
        control: controls[index],
        point: [50.0, y],
    }
}

fn wrapped(doc: &mut Document, body: NodeId, tag: &str, attributes: &[(&str, &str)]) -> NodeId {
    let wrapper = doc.create_element("div");
    doc.append(body, wrapper);
    let node = doc.create_element(tag);
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    doc.append(wrapper, node);
    node
}

/// `event` on the control removes it, or its parent, the first time.
pub fn remove_on(page: &Page, event: &str, removed: Removed) {
    let (doc, control) = (Rc::clone(&page.doc), page.control);
    let gone = match removed {
        Removed::Target => control,
        Removed::Parent => page.doc.borrow().parent(control).expect("wrapped"),
    };
    page.doc.borrow_mut().listen(
        control,
        event,
        Box::new(move |_: &mut Event| {
            if doc.borrow().contains(gone) {
                doc.borrow_mut().remove(gone);
            }
        }),
    );
}

/// The control's `pointerdown` captures the pointer.
pub fn capture_on_press(page: &Page) {
    page.doc.borrow_mut().listen(
        page.control,
        "pointerdown",
        Box::new(|e: &mut Event| e.set_pointer_capture()),
    );
}
