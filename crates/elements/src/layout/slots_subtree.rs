//! Rebuilding one box subtree in its slots (IU3 PR 1b): the subtree's
//! boxes are taken out, the new boxes take the slots their keys held in
//! any taken subtree, and the taken slots no new box takes become
//! vacant. Slots outside the taken subtrees are not touched.

use rustc_hash::FxHashMap;

use super::{Key, LayoutBox, Placed, keys, misplaced};

/// The boxes taken out of the rebuilt subtrees, with their keys, by
/// slot, and the slot each taken key held.
#[derive(Default)]
pub(crate) struct Taken {
    pub(crate) boxes: FxHashMap<usize, (Key, LayoutBox)>,
    slots: FxHashMap<Key, usize>,
}

impl Taken {
    /// The taken slots no new box took, with the key each held.
    pub(crate) fn unclaimed(&self) -> Vec<(usize, Key)> {
        self.slots.iter().map(|(&key, &slot)| (slot, key)).collect()
    }
}

impl Placed {
    /// Takes out the boxes of the subtree at `slot` into `taken`,
    /// leaving their slots vacant and keyless.
    pub(crate) fn take_subtree(&mut self, slot: usize, taken: &mut Taken) {
        let mut stack = vec![slot];
        while let Some(at) = stack.pop() {
            let old = std::mem::replace(&mut self.boxes[at], LayoutBox::vacant());
            let key = self.slots.keys[at]
                .take()
                .unwrap_or_else(|| panic!("slot {at} is in two rebuilt subtrees, or vacant"));
            self.slots.of.remove(&key);
            stack.extend(&old.children);
            taken.slots.insert(key, at);
            taken.boxes.insert(at, (key, old));
        }
    }

    /// Puts `built`, boxes in pre-order with the subtree's root first,
    /// into slots: a key `taken` holds takes its slot, a new key the
    /// lowest vacant slot or a new one. Returns each box's slot, in
    /// `built`'s order.
    ///
    /// # Panics
    /// A key that holds a slot outside the taken subtrees: a box that
    /// moved without a mark on its old parent.
    pub(crate) fn put_subtree(&mut self, built: Vec<LayoutBox>, taken: &mut Taken) -> Vec<usize> {
        let keys = keys(&built);
        let to: Vec<usize> = keys.iter().map(|key| self.slot_for(*key, taken)).collect();
        for (&key, &slot) in keys.iter().zip(&to) {
            self.hold(key, slot);
        }
        for (mut layout_box, &slot) in built.into_iter().zip(&to) {
            for child in &mut layout_box.children {
                *child = to[*child];
            }
            self.boxes[slot] = layout_box;
        }
        to
    }

    /// Records that `key` holds `slot`.
    fn hold(&mut self, key: Key, slot: usize) {
        let earlier = self.slots.of.insert(key, slot);
        assert!(earlier.is_none(), "box key {key:?} placed twice");
        self.slots.keys[slot] = Some(key);
    }

    /// The slot `key` takes, growing the boxes for a new one.
    fn slot_for(&mut self, key: Key, taken: &mut Taken) -> usize {
        assert!(
            self.slots.of(key).is_none(),
            "box key {key:?} is built again but holds a slot outside every rebuilt subtree; \
             a change missed a box mark (IU3)"
        );
        if let Some(slot) = taken.slots.remove(&key) {
            return slot;
        }
        if let Some(slot) = self.slots.free.pop() {
            return slot;
        }
        let slot = self.boxes.len();
        self.slots.keys.push(None);
        self.boxes.push(LayoutBox::vacant());
        slot
    }

    /// Frees the taken slots no new box took, once every subtree is put.
    pub(crate) fn free_taken(&mut self, taken: &Taken) {
        self.slots.free.extend(taken.slots.values().copied());
        self.slots.free.sort_unstable_by(|a, b| b.cmp(a));
        debug_assert_eq!(
            misplaced(&self.boxes),
            None,
            "a walk from the root reaches a vacant slot or misses a box"
        );
    }
}
