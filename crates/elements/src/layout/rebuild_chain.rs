//! What placement chains under each owner edited in place
//! (`Placement::chain_children`, 4h): the children it inserted, and the
//! other owners rebuilt directly under it. Its kept children keep their
//! slots and their parent, so they need no chaining again.

use vapors_dom::NodeId;

use super::super::boxes::Source;
use super::super::marks::Owner;
use super::super::slots::{Key, Placed};
use super::InsertedRoots;

/// Each owner edited in place, by slot, with the children placement
/// must chain to it (`Placement::chain_children`): those it inserted,
/// and every other owner rebuilt directly under it, whose box may have
/// come into flow (a row made static again) though the owner kept it.
pub(super) fn to_chain(
    placed: &Placed,
    source: &Source<'_>,
    (owners, slots, roots): (&[Owner], &[usize], &[InsertedRoots]),
) -> rustc_hash::FxHashMap<usize, Vec<usize>> {
    let mut chain: rustc_hash::FxHashMap<usize, Vec<usize>> = slots
        .iter()
        .zip(roots)
        .filter_map(|(&slot, roots)| Some((slot, roots.clone()?)))
        .collect();
    for (owner, &slot) in owners.iter().zip(slots) {
        let parent = box_parent(placed, source, owner.node);
        if let Some(children) = parent.and_then(|parent| chain.get_mut(&parent)) {
            children.push(slot);
        }
    }
    chain
}

/// The slot of the box `node`'s box is a child of: its nearest ancestor
/// element's that holds a box (an ancestor with `display: contents` or
/// none holds none).
fn box_parent(placed: &Placed, source: &Source<'_>, node: NodeId) -> Option<usize> {
    let mut at = source.arena.get(node, "box_parent").parent;
    while let Some(ancestor) = at {
        if let Some(slot) = placed.slots.of(Key::Element(ancestor)) {
            return Some(slot);
        }
        at = source.arena.get(ancestor, "box_parent").parent;
    }
    None
}
