//! A node of the graph and the arenas that hold nodes and owners.

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::error::NodeInfo;

/// Index plus generation, so a handle to a disposed node never aliases
/// the node that later reuses its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NodeId {
    pub index: u32,
    pub generation: u32,
}

/// Same shape as [`NodeId`], for owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct OwnerId {
    pub index: u32,
    pub generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum State {
    Clean,
    Check,
    Dirty,
}

/// The computation of a memo (returns whether its value changed) or of
/// an effect (always returns `true`). Shared so the runtime borrow can
/// be released while it runs.
pub(crate) type Compute = Rc<RefCell<dyn FnMut() -> bool>>;

/// The value cell of a signal (`T`) or memo (`Option<T>`).
pub(crate) type ValueCell = Rc<RefCell<dyn Any>>;

pub(crate) struct Node {
    /// Creation order; effects flush in ascending `seq`.
    pub seq: u64,
    pub info: NodeInfo,
    pub state: State,
    pub scheduled: bool,
    pub value: Option<ValueCell>,
    pub compute: Option<Compute>,
    pub sources: Vec<NodeId>,
    pub observers: Vec<NodeId>,
    /// The owner under which a memo or effect runs; disposed before
    /// each run and with the node.
    pub scope: Option<OwnerId>,
    pub owner: Option<OwnerId>,
}

pub(crate) type Cleanup = Box<dyn FnOnce()>;

pub(crate) struct OwnerNode {
    pub parent: Option<OwnerId>,
    pub children: Vec<OwnerId>,
    pub nodes: Vec<NodeId>,
    pub cleanups: Vec<Cleanup>,
}

pub(crate) struct Slot<T> {
    pub generation: u32,
    pub item: Option<T>,
}

/// A generational arena.
pub(crate) struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Arena<T> {
    /// Stores `item` and returns its index and generation.
    pub fn insert(&mut self, item: T) -> (u32, u32) {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.item = Some(item);
            return (index, slot.generation);
        }
        let index = u32::try_from(self.slots.len()).expect("fewer than 2^32 nodes");
        self.slots.push(Slot {
            generation: 0,
            item: Some(item),
        });
        (index, 0)
    }

    pub fn get(&self, index: u32, generation: u32) -> Option<&T> {
        let slot = self.slots.get(index as usize)?;
        (slot.generation == generation).then_some(slot.item.as_ref()?)
    }

    pub fn get_mut(&mut self, index: u32, generation: u32) -> Option<&mut T> {
        let slot = self.slots.get_mut(index as usize)?;
        (slot.generation == generation).then_some(slot.item.as_mut()?)
    }

    /// Removes the item and bumps the generation so stale ids miss.
    pub fn remove(&mut self, index: u32, generation: u32) -> Option<T> {
        let slot = self.slots.get_mut(index as usize)?;
        if slot.generation != generation {
            return None;
        }
        let item = slot.item.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(index);
        Some(item)
    }
}
