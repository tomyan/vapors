//! Shared by the a11y mirror tests (R6): the roles page in a shared
//! document, nodes found by role and name, and an update's records as
//! `role "name"` lines.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_elements::{AxMirrorNode, AxUpdate, Document};

use super::roles_page::page;
use super::viewport;

/// The roles page, updated, with its first (`every`) update taken, so
/// what a test changes next is what the next take holds.
pub fn taken_page() -> Rc<RefCell<Document>> {
    taken_model().0
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

/// A mirror as the tests model one: each parent's children in order,
/// built from the first take and kept in step by the takes after, as
/// the web mirror keeps its DOM (`docs/plans/r6.md`): records first,
/// then placements in their order, then removals.
pub struct Model {
    children: HashMap<Option<NodeId>, Vec<NodeId>>,
    parent_of: HashMap<NodeId, Option<NodeId>>,
}

impl Model {
    /// A mirror built from a take of every node: each appended to its
    /// parent, in order.
    pub fn from_every(update: &AxUpdate) -> Self {
        assert!(update.every, "the model starts from every node");
        let mut model = Self {
            children: HashMap::new(),
            parent_of: HashMap::new(),
        };
        for node in &update.nodes {
            model.attach(node.node, node.parent, None);
        }
        model
    }

    /// Applies `update`.
    pub fn apply(&mut self, update: &AxUpdate) {
        for place in &update.placed {
            self.detach(place.node);
            self.attach(place.node, place.parent, place.before);
        }
        for &node in &update.removed {
            self.detach(node);
            self.children.remove(&Some(node));
        }
    }

    /// `parent`'s children, in order.
    pub fn children(&self, parent: Option<NodeId>) -> Vec<NodeId> {
        self.children.get(&parent).cloned().unwrap_or_default()
    }

    fn attach(&mut self, node: NodeId, parent: Option<NodeId>, before: Option<NodeId>) {
        let siblings = self.children.entry(parent).or_default();
        let at = match before {
            Some(before) => siblings
                .iter()
                .position(|&s| s == before)
                .unwrap_or_else(|| {
                    panic!("{node:?} placed before {before:?}, not under {parent:?}")
                }),
            None => siblings.len(),
        };
        siblings.insert(at, node);
        self.parent_of.insert(node, parent);
    }

    fn detach(&mut self, node: NodeId) {
        if let Some(parent) = self.parent_of.remove(&node)
            && let Some(siblings) = self.children.get_mut(&parent)
        {
            siblings.retain(|&s| s != node);
        }
    }
}

/// The roles page with its first take, and a model built from it.
pub fn taken_model() -> (Rc<RefCell<Document>>, Model) {
    let doc = Rc::new(RefCell::new(page()));
    let first = doc
        .borrow_mut()
        .take_ax_update()
        .expect("an updated document")
        .expect("the first take holds every node");
    let model = Model::from_every(&first);
    (doc, model)
}
