//! The page's a11y mirror (R6, `docs/plans/r6.md`): one element per
//! node of the document's accessibility tree, kept in step by the
//! updates the app hands the platform after each frame. An update is
//! applied as records, then placements, then removals, as the document
//! orders them; `every` is a reconcile keyed by node, never a rebuild,
//! so DOM focus and a reader's cursor survive it. The DOM is behind
//! [`MirrorDom`]: the logic runs on a fake natively (its tests) and on
//! `web_sys` on the page (`a11y_mirror_web.rs`). An update the mirror
//! cannot apply as given (a node it does not hold, a record left
//! unplaced) is refused naming the node, never approximated. Focus
//! (`AxUpdate::focus`) is PR 6's and is not applied here.

use std::collections::{HashMap, HashSet};

use vapors_a11y::{AxMirrorNode, AxPlace, AxUpdate, NodeId};

use crate::a11y_mirror_attrs::{attributes, text};

/// The DOM operations a mirror needs.
pub(crate) trait MirrorDom {
    /// An element handle.
    type Element: Clone;
    /// A new, detached `<div>`.
    ///
    /// # Errors
    /// The page could not make one.
    fn create(&mut self) -> Result<Self::Element, String>;
    fn set_attribute(&mut self, element: &Self::Element, name: &str, value: &str);
    fn remove_attribute(&mut self, element: &Self::Element, name: &str);
    /// The element's own text, kept apart from its child elements.
    fn set_text(&mut self, element: &Self::Element, text: &str);
    /// `child` as `parent`'s last child; moved there if it is in the tree.
    fn append(&mut self, parent: &Self::Element, child: &Self::Element);
    /// `child` under `parent` before `before`; moved there if in the tree.
    fn insert_before(
        &mut self,
        parent: &Self::Element,
        child: &Self::Element,
        before: &Self::Element,
    );
    /// `element` and its subtree out of the tree.
    fn remove(&mut self, element: &Self::Element);
}

/// The mirror's elements by node, under its root.
pub(crate) struct Mirror<D: MirrorDom> {
    root: D::Element,
    elements: HashMap<u64, D::Element>,
    /// Nodes whose element a record made and no placement has put in.
    unplaced: Vec<NodeId>,
}

impl<D: MirrorDom> Mirror<D> {
    /// An empty mirror under `root`.
    pub(crate) fn new(root: D::Element) -> Self {
        Self {
            root,
            elements: HashMap::new(),
            unplaced: Vec::new(),
        }
    }

    /// Applies `update`.
    ///
    /// # Errors
    /// A node the update names that the mirror does not hold, a record
    /// the update left unplaced, or a record the mirror has no
    /// attributes for; each names the node.
    pub(crate) fn apply(&mut self, dom: &mut D, update: &AxUpdate) -> Result<(), String> {
        if update.every {
            return self.reconcile(dom, &update.nodes);
        }
        for node in &update.nodes {
            self.record(dom, node)?;
        }
        for place in &update.placed {
            self.place(dom, place)?;
        }
        for &node in &update.removed {
            self.remove(dom, node)?;
        }
        self.all_placed()
    }

    /// The node's element, made if new, with the record's attributes.
    fn record(&mut self, dom: &mut D, node: &AxMirrorNode) -> Result<(), String> {
        let element = self.held_or_made(dom, node.node)?;
        for (name, value) in attributes(node)? {
            match value {
                Some(value) => dom.set_attribute(&element, name, &value),
                None => dom.remove_attribute(&element, name),
            }
        }
        dom.set_text(&element, text(node));
        Ok(())
    }

    /// `node`'s element, or a new one, detached until placed.
    fn held_or_made(&mut self, dom: &mut D, node: NodeId) -> Result<D::Element, String> {
        if let Some(element) = self.elements.get(&node.0) {
            return Ok(element.clone());
        }
        let element = dom.create()?;
        self.elements.insert(node.0, element.clone());
        self.unplaced.push(node);
        Ok(element)
    }

    fn place(&mut self, dom: &mut D, place: &AxPlace) -> Result<(), String> {
        let child = self.element(place.node, "places")?;
        let parent = self.parent(place.parent, place.node)?;
        match place.before {
            Some(before) => {
                let before = self.element(before, "places before")?;
                dom.insert_before(&parent, &child, &before);
            }
            None => dom.append(&parent, &child),
        }
        self.unplaced.retain(|node| *node != place.node);
        Ok(())
    }

    fn remove(&mut self, dom: &mut D, node: NodeId) -> Result<(), String> {
        let element = self
            .elements
            .remove(&node.0)
            .ok_or_else(|| lagged("removes", node))?;
        dom.remove(&element);
        Ok(())
    }

    /// Every node's element under its parent in order, reused when
    /// held; the elements of nodes not in `nodes` removed.
    fn reconcile(&mut self, dom: &mut D, nodes: &[AxMirrorNode]) -> Result<(), String> {
        for node in nodes {
            self.record(dom, node)?;
            let parent = self.parent(node.parent, node.node)?;
            let element = self.element(node.node, "holds")?;
            dom.append(&parent, &element);
        }
        self.unplaced.clear();
        let kept: HashSet<u64> = nodes.iter().map(|node| node.node.0).collect();
        let gone: Vec<u64> = self
            .elements
            .keys()
            .filter(|id| !kept.contains(id))
            .copied()
            .collect();
        for id in gone {
            if let Some(element) = self.elements.remove(&id) {
                dom.remove(&element);
            }
        }
        Ok(())
    }

    fn element(&self, node: NodeId, what: &str) -> Result<D::Element, String> {
        self.elements
            .get(&node.0)
            .cloned()
            .ok_or_else(|| lagged(what, node))
    }

    /// `parent`'s element, or the root for a top node.
    fn parent(&self, parent: Option<NodeId>, node: NodeId) -> Result<D::Element, String> {
        match parent {
            None => Ok(self.root.clone()),
            Some(parent) => self.elements.get(&parent.0).cloned().ok_or_else(|| {
                format!(
                    "the a11y update puts node {} under node {}, which the mirror does not hold: \
                     the update lagged the document",
                    node.0, parent.0
                )
            }),
        }
    }

    fn all_placed(&self) -> Result<(), String> {
        if self.unplaced.is_empty() {
            return Ok(());
        }
        let nodes: Vec<String> = self
            .unplaced
            .iter()
            .map(|node| node.0.to_string())
            .collect();
        Err(format!(
            "the a11y update records new node(s) {} without placing them, so a reader would \
             not reach them: the document's take must place every new node",
            nodes.join(", ")
        ))
    }
}

fn lagged(what: &str, node: NodeId) -> String {
    format!(
        "the a11y update {what} node {}, which the mirror does not hold: the update lagged \
         the document",
        node.0
    )
}
