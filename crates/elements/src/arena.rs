//! Node storage. A [`NodeId`] packs a slot index (low 32 bits) and the
//! slot's generation (high 32 bits), so an id kept after its node was
//! freed is detected rather than read as whatever reused the slot.

use vapors_dom::NodeId;

use crate::node::Node;

struct Slot {
    generation: u32,
    node: Option<Node>,
}

/// The slots, and the free list of empty ones.
#[derive(Default)]
pub(crate) struct Arena {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

impl Arena {
    pub(crate) fn insert(&mut self, node: Node) -> NodeId {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            return id(index, slot.generation);
        }
        let index = u32::try_from(self.slots.len()).expect("fewer than 2^32 nodes");
        self.slots.push(Slot {
            generation: 0,
            node: Some(node),
        });
        id(index, 0)
    }

    /// Frees the node's slot; its id is stale from now on.
    pub(crate) fn free(&mut self, node: NodeId) {
        let (index, _) = self.slot_of(node, "free");
        let slot = &mut self.slots[index];
        slot.node = None;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(index_of(node));
    }

    /// The node, or a panic naming the call and the id when it is unknown
    /// or stale: only a bug in generated code or the runtime gets here.
    pub(crate) fn get(&self, node: NodeId, call: &str) -> &Node {
        let (index, _) = self.slot_of(node, call);
        self.slots[index].node.as_ref().expect("checked by slot_of")
    }

    /// As [`Arena::get`], mutably.
    pub(crate) fn get_mut(&mut self, node: NodeId, call: &str) -> &mut Node {
        let (index, _) = self.slot_of(node, call);
        self.slots[index].node.as_mut().expect("checked by slot_of")
    }

    /// Whether `node` names a live node.
    pub(crate) fn contains(&self, node: NodeId) -> bool {
        self.slots
            .get(index_of(node) as usize)
            .is_some_and(|slot| slot.generation == generation_of(node) && slot.node.is_some())
    }

    /// One past the highest slot index: the size of per-node side tables.
    pub(crate) fn capacity(&self) -> usize {
        self.slots.len()
    }

    fn slot_of(&self, node: NodeId, call: &str) -> (usize, u32) {
        assert!(
            self.contains(node),
            "{call}: node {node:?} (slot {}, generation {}) is not in the document; it was removed, or never created by it",
            index_of(node),
            generation_of(node)
        );
        (index_of(node) as usize, generation_of(node))
    }
}

/// The slot index of a node: the key of per-node side tables.
pub(crate) fn index_of(node: NodeId) -> u32 {
    (node.0 & 0xffff_ffff) as u32
}

fn generation_of(node: NodeId) -> u32 {
    (node.0 >> 32) as u32
}

fn id(index: u32, generation: u32) -> NodeId {
    NodeId(u64::from(generation) << 32 | u64::from(index))
}
