//! The changed owners a rebuild hands to layout (`isolated.rs`): each
//! with its slot, its ancestors' slots and what the box it replaced
//! held.

use vapors_dom::NodeId;

use super::super::boxes::LayoutBox;
use super::super::isolated::{Before, Changed};
use super::super::slots::{Key, Placed, Taken};
use crate::arena::Arena;

/// Each changed owner for layout to settle: its slot, its ancestors'
/// slots nearest first, and what the taken box of its key in its slot
/// held; nothing for an owner `may_keep` refuses (restyled, with a
/// calc in its style whose handle cannot be compared, `keep.rs`).
pub(super) fn changed_owners(
    placed: &Placed,
    arena: &Arena,
    changed: &[(NodeId, usize)],
    (mut taken, may_keep): (Taken, &dyn Fn(NodeId) -> bool),
) -> Vec<Changed> {
    let before = |old: (Key, LayoutBox)| match old {
        (Key::Element(_), old) => Some(Before::of(old)),
        (Key::Text(..), _) => None,
    };
    changed
        .iter()
        .map(|&(owner, slot)| Changed {
            slot,
            ancestors: ancestor_slots(placed, ancestors(arena, owner)),
            before: taken
                .boxes
                .remove(&slot)
                .filter(|_| may_keep(owner))
                .and_then(before),
        })
        .collect()
}

/// The slots of those of `ancestors` that hold a box. Only element
/// boxes have children, so these are every box above a rebuilt subtree.
pub(super) fn ancestor_slots(
    placed: &Placed,
    ancestors: impl Iterator<Item = NodeId>,
) -> Vec<usize> {
    ancestors
        .filter_map(|ancestor| placed.slots.of(Key::Element(ancestor)))
        .collect()
}

/// The elements above `node`, nearest first.
fn ancestors(arena: &Arena, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    let parent = |&node: &NodeId| arena.get(node, "ancestor_slots").parent;
    std::iter::successors(parent(&node), parent)
}
