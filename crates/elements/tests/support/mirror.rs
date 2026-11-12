//! Shared by the a11y mirror tests (R6): the roles page in a shared
//! document, nodes found by role and name, and an update's records as
//! `role "name"` lines.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{AxMirrorNode, AxUpdate, Document};

use super::roles_page::{STYLE, labelled, page};
use super::{document, element, text, viewport};

/// A page holding one `role` element with `attributes` and a button
/// after it, updated: what a refusal test takes.
pub fn page_with(role: &str, attributes: &[(&str, &str)]) -> Document {
    let mut doc = document();
    let body = doc.body();
    let row = element(&mut doc, body, "div", "row");
    let node = labelled(&mut doc, row, "div", &[("role", role)]);
    for (name, value) in attributes {
        doc.set_attribute(node, name, value);
    }
    text(&mut doc, node, "Title");
    element(&mut doc, row, "button", "");
    doc.add_stylesheet("roles.css", STYLE).expect("parses");
    doc.update(viewport(640.0, 400.0)).expect("updates");
    doc
}

/// The roles page, updated, with its first (`every`) update taken, so
/// what a test changes next is what the next take holds.
pub fn taken_page() -> Rc<RefCell<Document>> {
    let doc = Rc::new(RefCell::new(page()));
    let first = doc
        .borrow_mut()
        .take_ax_update()
        .expect("an updated document")
        .expect("the first take holds every node");
    assert!(first.every, "the first take is every node");
    doc
}

/// Updates `doc` at the roles page's viewport and takes the update.
pub fn updated_take(doc: &Rc<RefCell<Document>>) -> Option<AxUpdate> {
    let mut doc = doc.borrow_mut();
    doc.update(viewport(640.0, 400.0)).expect("updates");
    doc.take_ax_update().expect("an updated document")
}

/// The node with `role` and `name` in `doc`'s tree.
pub fn node_of(doc: &Rc<RefCell<Document>>, role: &str, name: &str) -> NodeId {
    doc.borrow()
        .ax_find(role, name, None)
        .unwrap_or_else(|e| panic!("{role} \"{name}\": {e}"))
        .node
}

/// Each record as `role "name"`, sorted: an update's order is not
/// document order, and a mirror does not need it to be.
pub fn labels(nodes: &[AxMirrorNode]) -> Vec<String> {
    let mut labels: Vec<String> = nodes
        .iter()
        .map(|node| format!("{} \"{}\"", node.role, node.name))
        .collect();
    labels.sort();
    labels
}
