//! Box marks (IU3): the elements whose boxes may have changed since
//! the last update, so the next rebuilds their boxes and keeps the
//! rest. Each mark resolves to its owner, the nearest element, itself
//! or an ancestor, that held a box slot last update and still makes a
//! box of its own; the owner's subtree is built again.

use std::collections::{HashMap, HashSet};

use taffy::Display;
use vapors_dom::NodeId;

use super::slots::{Key, Slots};
use crate::arena::Arena;
use crate::style::Used;

/// What changed since the last update: every box, or the boxes of
/// these elements.
#[derive(Default)]
pub(crate) struct Marks {
    pub(crate) all: bool,
    nodes: HashSet<NodeId>,
}

impl Marks {
    /// Marks `node`'s box, or the box of the element that owns it.
    pub(crate) fn mark(&mut self, node: NodeId) {
        self.nodes.insert(node);
    }

    /// What the next update builds again: every box when a mark
    /// reaches the root with no owner on the way (the root lost its
    /// box), or the subtrees of these elements under `root`, none
    /// inside another, in slot order so new keys take the same slots
    /// on every run. A mark on a node freed or detached since has no
    /// owner: the mark `remove` or `insert_before` put on its old
    /// parent covers its old boxes.
    pub(crate) fn rebuild(
        &self,
        arena: &Arena,
        used: &HashMap<NodeId, Used>,
        slots: &Slots,
        root: NodeId,
    ) -> Rebuild {
        let mut owners = HashSet::new();
        for &node in &self.nodes {
            match owner(arena, used, slots, root, node) {
                Owner::Element(owner) => _ = owners.insert(owner),
                Owner::None => return Rebuild::Every,
                Owner::Detached => {}
            }
        }
        Rebuild::Owners(outermost(arena, slots, &owners, root))
    }
}

/// What an update builds again.
#[derive(Debug, PartialEq)]
pub(crate) enum Rebuild {
    /// Every box.
    Every,
    /// The subtrees of these elements, in the slots they hold.
    Owners(Vec<NodeId>),
}

/// Where a mark resolves.
enum Owner {
    /// To this element, which builds the marked node's boxes.
    Element(NodeId),
    /// To no element: the walk reached the root without one.
    None,
    /// Nowhere: the node is freed or not under the root.
    Detached,
}

/// The owners under `root` with no other owner above them, by slot.
fn outermost(arena: &Arena, slots: &Slots, owners: &HashSet<NodeId>, root: NodeId) -> Vec<NodeId> {
    let mut outermost: Vec<NodeId> = owners
        .iter()
        .copied()
        .filter(|&node| placement(arena, node, owners, root) == Placement::Outermost)
        .collect();
    outermost.sort_by_key(|&node| slots.of(Key::Element(node)));
    outermost
}

/// The nearest element at or above `node` with a slot and a box of
/// its own.
fn owner(
    arena: &Arena,
    used: &HashMap<NodeId, Used>,
    slots: &Slots,
    root: NodeId,
    node: NodeId,
) -> Owner {
    let mut at = Some(node).filter(|&node| arena.contains(node));
    while let Some(node) = at {
        if boxed(used, node) && slots.of(Key::Element(node)).is_some() {
            return Owner::Element(node);
        }
        if node == root {
            return Owner::None;
        }
        at = arena.get(node, "owner").parent;
    }
    Owner::Detached
}

/// Whether `node` makes a box of its own: not `display: none` and not
/// inline.
fn boxed(used: &HashMap<NodeId, Used>, node: NodeId) -> bool {
    used.get(&node)
        .is_some_and(|style| style.layout.display != Display::None && !style.inline)
}

/// Where an owner sits among the others.
#[derive(PartialEq)]
enum Placement {
    /// Under `root`, with no other owner above it.
    Outermost,
    /// Under another owner, which builds its boxes.
    Inside,
    /// Not under `root`: nothing shows it.
    Detached,
}

fn placement(arena: &Arena, node: NodeId, owners: &HashSet<NodeId>, root: NodeId) -> Placement {
    let mut at = node;
    while let Some(ancestor) = arena.get(at, "owner").parent {
        if owners.contains(&ancestor) {
            return Placement::Inside;
        }
        at = ancestor;
    }
    if at == root {
        Placement::Outermost
    } else {
        Placement::Detached
    }
}

#[cfg(test)]
#[path = "marks_tests.rs"]
mod tests;
