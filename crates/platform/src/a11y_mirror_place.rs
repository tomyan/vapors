//! Where the mirror's elements go (`a11y_mirror_dom.rs`): a placement
//! puts a node's element under its parent before a sibling, a removal
//! takes one out, and the reconcile of `every` moves only the elements
//! out of place, as the order model says (`a11y_mirror_order.rs`).

use std::collections::HashSet;

use vapors_a11y::{AxMirrorNode, AxPlace, NodeId};

use crate::a11y_mirror_dom::{Mirror, MirrorDom, lagged};
use crate::a11y_mirror_order::Move;

impl<D: MirrorDom> Mirror<D> {
    pub(crate) fn place(&mut self, dom: &mut D, place: &AxPlace) -> Result<(), String> {
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

    pub(crate) fn remove(&mut self, dom: &mut D, node: NodeId) -> Result<(), String> {
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
    pub(crate) fn reconcile(&mut self, dom: &mut D, nodes: &[AxMirrorNode]) -> Result<(), String> {
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
}
