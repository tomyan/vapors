//! Which slot index each mesh node holds, and the world version written
//! there. A removed node's index is reused by the next node added, so
//! the slot buffer grows only with the most mesh nodes drawn at once.

use crate::FrameError;
use std::collections::{HashMap, HashSet};
use vapors_scene3d::NodeId;

#[derive(Default)]
pub(crate) struct SlotTable {
    of: HashMap<NodeId, Slot>,
    free: Vec<u32>,
    used: u32,
}

/// Where a node's uniform sits, and the world version it holds.
struct Slot {
    index: u32,
    written: Option<u64>,
}

impl SlotTable {
    /// Frees the slots of nodes not in `nodes`, then gives each of
    /// `nodes` without a slot one, a freed index first, unwritten.
    /// Returns how many slots the buffer must hold.
    pub(crate) fn assign(&mut self, nodes: &[NodeId]) -> u32 {
        self.free_absent(nodes);
        for node in nodes {
            if !self.of.contains_key(node) {
                let index = self.next_index();
                self.of.insert(
                    *node,
                    Slot {
                        index,
                        written: None,
                    },
                );
            }
        }
        self.used
    }

    fn free_absent(&mut self, nodes: &[NodeId]) {
        let live: HashSet<&NodeId> = nodes.iter().collect();
        let free = &mut self.free;
        self.of.retain(|node, slot| {
            let keep = live.contains(node);
            if !keep {
                free.push(slot.index);
            }
            keep
        });
    }

    fn next_index(&mut self) -> u32 {
        self.free.pop().unwrap_or_else(|| {
            self.used += 1;
            self.used - 1
        })
    }

    /// Whether `node`'s slot holds world version `version`.
    pub(crate) fn holds(&self, node: NodeId, version: u64) -> bool {
        self.of
            .get(&node)
            .is_some_and(|slot| slot.written == Some(version))
    }

    /// `node`'s slot index; `None` before [`SlotTable::assign`].
    pub(crate) fn index(&self, node: NodeId) -> Option<u32> {
        self.of.get(&node).map(|slot| slot.index)
    }

    /// Records that `node`'s slot holds `version`; returns its index,
    /// `None` before [`SlotTable::assign`].
    pub(crate) fn mark_written(&mut self, node: NodeId, version: u64) -> Option<u32> {
        let slot = self.of.get_mut(&node)?;
        slot.written = Some(version);
        Some(slot.index)
    }

    /// Every slot unwritten, as after the buffer is replaced.
    pub(crate) fn forget_written(&mut self) {
        self.of.values_mut().for_each(|slot| slot.written = None);
    }
}

/// Slots a buffer for `needed` slots of `stride` bytes holds: the next
/// power of two, cut to what `max_buffer_size` allows.
///
/// # Errors
///
/// [`FrameError::TooManySlots`] when `needed` slots pass `max_buffer_size`.
pub(crate) fn capacity_for(
    needed: u32,
    stride: u32,
    max_buffer_size: u64,
) -> Result<u32, FrameError> {
    let fits = u32::try_from(max_buffer_size / u64::from(stride)).unwrap_or(u32::MAX);
    if needed > fits {
        return Err(FrameError::TooManySlots {
            slots: needed,
            stride,
            limit: max_buffer_size,
        });
    }
    Ok(needed.checked_next_power_of_two().unwrap_or(fits).min(fits))
}

#[cfg(test)]
#[path = "slot_table_tests.rs"]
mod tests;
