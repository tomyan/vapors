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

    /// The elements under `root` whose subtrees are built again, none
    /// inside another. A mark on a node freed or detached since, or
    /// under no slotted box, has no owner: the mark `remove` or
    /// `insert_before` put on its old parent covers its old boxes.
    pub(crate) fn owners(
        &self,
        arena: &Arena,
        used: &HashMap<NodeId, Used>,
        slots: &Slots,
        root: NodeId,
    ) -> Vec<NodeId> {
        let owners: HashSet<NodeId> = self
            .nodes
            .iter()
            .filter_map(|&node| owner(arena, used, slots, node))
            .collect();
        owners
            .iter()
            .copied()
            .filter(|&node| placement(arena, node, &owners, root) == Placement::Outermost)
            .collect()
    }
}

/// The nearest element at or above `node` with a slot and a box of
/// its own: not `display: none` and not inline.
fn owner(
    arena: &Arena,
    used: &HashMap<NodeId, Used>,
    slots: &Slots,
    node: NodeId,
) -> Option<NodeId> {
    let mut at = Some(node).filter(|&node| arena.contains(node));
    while let Some(node) = at {
        let boxed = used
            .get(&node)
            .is_some_and(|style| style.layout.display != Display::None && !style.inline);
        if boxed && slots.of(Key::Element(node)).is_some() {
            return Some(node);
        }
        at = arena.get(node, "owner").parent;
    }
    None
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
