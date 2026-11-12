//! The page's a11y mirror (R6, `docs/plans/r6.md`): one element per
//! node of the document's accessibility tree, kept in step by the
//! updates the app hands the platform after each frame. An update is
//! applied as records, then placements, then removals, as the document
//! orders them; `every` is a reconcile keyed by node that moves only
//! the elements out of place (`a11y_mirror_order.rs`), since Chromium
//! resets a scroll container's offset and drops DOM focus when an
//! element is appended again. The DOM is behind [`MirrorDom`]: the
//! logic runs on a fake natively (its tests) and on `web_sys` on the
//! page (`a11y_mirror_web.rs`). An update the mirror cannot apply as
//! given (a node it does not hold, a record left unplaced, a placement
//! the page refuses) is refused naming the node, never approximated.
//! The record last applied to each node is kept, so a record writes
//! only what changed (`a11y_mirror_write.rs`); scroll offsets are
//! written last, once every element is in place
//! (`a11y_mirror_scroll.rs`), since a detached or re-appended element
//! reads 0. Focus (`AxUpdate::focus`) is PR 6's and is not applied here.

use std::collections::{HashMap, HashSet};

use vapors_a11y::{AxMirrorNode, AxPlace, AxUpdate, NodeId};

use crate::a11y_mirror_order::{Move, Order};
use crate::a11y_mirror_write::write;

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
    ///
    /// # Errors
    /// The page refused the placement.
    fn append(&mut self, parent: &Self::Element, child: &Self::Element) -> Result<(), String>;
    /// `child` under `parent` before `before`; moved there if in the tree.
    ///
    /// # Errors
    /// The page refused the placement.
    fn insert_before(
        &mut self,
        parent: &Self::Element,
        child: &Self::Element,
        before: &Self::Element,
    ) -> Result<(), String>;
    /// `element` and its subtree out of the tree.
    fn remove(&mut self, element: &Self::Element);
    /// Scrolls `element`'s content to `offset` (`[x, y]`, CSS pixels);
    /// nothing for a detached element, as on the page.
    fn set_scroll(&mut self, element: &Self::Element, offset: [f32; 2]);
}

/// The mirror's elements by node, under its root.
pub(crate) struct Mirror<D: MirrorDom> {
    root: D::Element,
    pub(crate) elements: HashMap<u64, D::Element>,
    /// The record last applied to each node's element.
    pub(crate) records: HashMap<u64, AxMirrorNode>,
    /// Where each element is.
    pub(crate) order: Order,
    /// Nodes whose element a record made and no placement has put in.
    unplaced: Vec<NodeId>,
    /// The nodes whose element this apply made or moved, so the scroll
    /// offsets of the scroll containers in them are written again.
    pub(crate) moved: HashSet<u64>,
    /// The scroll containers whose record changed its offset, or is new.
    pub(crate) scroll_changed: HashSet<u64>,
}

impl<D: MirrorDom> Mirror<D> {
    /// An empty mirror under `root`.
    pub(crate) fn new(root: D::Element) -> Self {
        Self {
            root,
            elements: HashMap::new(),
            records: HashMap::new(),
            order: Order::default(),
            unplaced: Vec::new(),
            moved: HashSet::new(),
            scroll_changed: HashSet::new(),
        }
    }

    /// Applies `update`.
    ///
    /// # Errors
    /// A node the update names that the mirror does not hold, a record
    /// the update left unplaced, a record the mirror has no attributes
    /// for, or a placement the page refused; each names the node.
    pub(crate) fn apply(&mut self, dom: &mut D, update: &AxUpdate) -> Result<(), String> {
        self.moved.clear();
        if update.every {
            self.reconcile(dom, &update.nodes)?;
        } else {
            self.increment(dom, update)?;
        }
        self.write_scrolls(dom);
        Ok(())
    }

    fn increment(&mut self, dom: &mut D, update: &AxUpdate) -> Result<(), String> {
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

    /// The node's element, made if new, with what the record changed
    /// written to it.
    fn record(&mut self, dom: &mut D, node: &AxMirrorNode) -> Result<(), String> {
        let element = self.held_or_made(dom, node.node)?;
        let last = self.records.get(&node.node.0);
        if write(dom, &element, node, last)? {
            self.scroll_changed.insert(node.node.0);
        }
        self.records.insert(node.node.0, node.clone());
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
        let (parent, before) = (place.parent.map(|p| p.0), place.before.map(|b| b.0));
        self.put(dom, place.node.0, parent, before)?;
        self.order.place(place.node.0, parent, before);
        self.unplaced.retain(|node| *node != place.node);
        Ok(())
    }

    /// `node`'s element under `parent`'s before `before`'s, or last.
    fn put(
        &mut self,
        dom: &mut D,
        node: u64,
        parent: Option<u64>,
        before: Option<u64>,
    ) -> Result<(), String> {
        let child = self.element(NodeId(node), "places")?;
        let parent = self.parent(parent.map(NodeId), NodeId(node))?;
        let placed = match before {
            Some(before) => {
                let before = self.element(NodeId(before), "places before")?;
                dom.insert_before(&parent, &child, &before)
            }
            None => dom.append(&parent, &child),
        };
        placed
            .map_err(|refused| format!("the page refused to place a11y node {node}: {refused}"))?;
        self.moved.insert(node);
        Ok(())
    }

    fn remove(&mut self, dom: &mut D, node: NodeId) -> Result<(), String> {
        let element = self
            .elements
            .remove(&node.0)
            .ok_or_else(|| lagged("removes", node))?;
        self.records.remove(&node.0);
        self.order.remove(node.0);
        dom.remove(&element);
        Ok(())
    }

    /// Every node's record written, the elements of nodes not in
    /// `nodes` removed, then the elements out of place moved.
    fn reconcile(&mut self, dom: &mut D, nodes: &[AxMirrorNode]) -> Result<(), String> {
        for node in nodes {
            self.record(dom, node)?;
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
            self.remove(dom, NodeId(id))?;
        }
        let desired: Vec<(u64, Option<u64>)> = nodes
            .iter()
            .map(|node| (node.node.0, node.parent.map(|p| p.0)))
            .collect();
        for Move {
            node,
            parent,
            before,
        } in self.order.reconcile(&desired)
        {
            self.put(dom, node, parent, before)?;
        }
        Ok(())
    }

    pub(crate) fn element(&self, node: NodeId, what: &str) -> Result<D::Element, String> {
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
