//! Rebuilding one box subtree in its slots (IU3 PR 1b): the subtree's
//! boxes are taken out, the new boxes take the slots their keys held in
//! any taken subtree, and the taken slots no new box takes become
//! vacant. Slots outside the taken subtrees are not touched.

use rustc_hash::FxHashMap;

use super::super::owners::KeptChildren;
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

    /// The slot `key` held, when its box was taken and not put back.
    pub(crate) fn held(&self, key: Key) -> Option<usize> {
        self.slots.get(&key).copied()
    }
}

impl Placed {
    /// Takes out the boxes of the subtree at `slot` into `taken`,
    /// leaving their slots vacant and keyless.
    pub(crate) fn take_subtree(&mut self, slot: usize, taken: &mut Taken) {
        let mut stack = vec![slot];
        while let Some(at) = stack.pop() {
            stack.extend(self.take_box(at, taken));
        }
    }

    /// Takes out the box at `slot` and the subtrees of its children
    /// other than `kept`'s, which keep their slots and boxes. A kept
    /// child that is an owner too may be taken already, so its key is
    /// read from `taken` when its slot has none.
    pub(crate) fn take_shallow(&mut self, slot: usize, kept: &KeptChildren, taken: &mut Taken) {
        for child in self.take_box(slot, taken) {
            let held = |key| matches!(key, Key::Element(node) if kept.get(&node) == Some(&child));
            let key =
                (self.slots.key(child)).or_else(|| taken.boxes.get(&child).map(|(key, _)| *key));
            if !key.is_some_and(held) {
                self.take_subtree(child, taken);
            }
        }
    }

    /// Takes out the box at `slot` into `taken`; its children.
    pub(super) fn take_box(&mut self, at: usize, taken: &mut Taken) -> Vec<usize> {
        let old = std::mem::replace(&mut self.boxes[at], LayoutBox::vacant());
        let key = self.slots.keys[at]
            .take()
            .unwrap_or_else(|| panic!("slot {at} is in two rebuilt subtrees, or vacant"));
        self.slots.of.remove(&key);
        let children = old.children.clone();
        taken.slots.insert(key, at);
        taken.boxes.insert(at, (key, old));
        children
    }

    /// Puts `built`, boxes in pre-order with the subtree's root first,
    /// into slots: a key `taken` holds takes its slot, a new key the
    /// lowest vacant slot or a new one. A box at an index `kept` names
    /// is a placeholder for the kept box in the slot it gives, which
    /// stays as it is. Returns the slot of each box built, in `built`'s
    /// order.
    ///
    /// # Panics
    /// A key that holds a slot outside the taken subtrees: a box that
    /// moved without a mark on its old parent.
    pub(crate) fn put_subtree(
        &mut self,
        built: Vec<LayoutBox>,
        kept: &FxHashMap<usize, usize>,
        taken: &mut Taken,
    ) -> Vec<usize> {
        let keys = keys(&built);
        let to = self.targets(&keys, kept, taken);
        let mut put = Vec::with_capacity(built.len() - kept.len());
        for (index, layout_box) in built.into_iter().enumerate() {
            if !kept.contains_key(&index) {
                self.put_box(layout_box, (keys[index], to[index]), &to);
                put.push(to[index]);
            }
        }
        put
    }

    /// The slot each of the boxes with `keys` goes to: a placeholder's
    /// kept slot, else the slot its key takes.
    fn targets(
        &mut self,
        keys: &[Key],
        kept: &FxHashMap<usize, usize>,
        taken: &mut Taken,
    ) -> Vec<usize> {
        (keys.iter().enumerate())
            .map(|(index, &key)| match kept.get(&index) {
                Some(&slot) => slot,
                None => self.slot_for(key, taken),
            })
            .collect()
    }

    /// Puts `layout_box`, with `key`, in `slot`, its children's indices
    /// mapped to slots by `to`.
    fn put_box(&mut self, mut layout_box: LayoutBox, (key, slot): (Key, usize), to: &[usize]) {
        self.hold(key, slot);
        // a container built again starts its lane afresh
        self.lanes.remove(slot);
        for child in &mut layout_box.children {
            *child = to[*child];
        }
        self.boxes[slot] = layout_box;
    }

    /// Records that `key` holds `slot`.
    pub(super) fn hold(&mut self, key: Key, slot: usize) {
        let earlier = self.slots.of.insert(key, slot);
        assert!(earlier.is_none(), "box key {key:?} placed twice");
        self.slots.keys[slot] = Some(key);
    }

    /// The slot `key` takes, growing the boxes for a new one.
    pub(super) fn slot_for(&mut self, key: Key, taken: &mut Taken) -> usize {
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
        // grown by a quarter, not doubled (`slot_list.rs`); a box is about
        // 2.8 KB. Each growth copies every box; chunked storage, as the
        // blocks of #288, is the follow-up if that shows in inserts.
        crate::slot_list::reserve(&mut self.slots.keys, slot + 1);
        self.slots.keys.push(None);
        crate::slot_list::reserve(&mut self.boxes, slot + 1);
        self.boxes.push(LayoutBox::vacant());
        slot
    }

    /// Frees the taken slots no new box took, once every subtree is put.
    pub(crate) fn free_taken(&mut self, taken: &Taken) {
        for slot in taken.slots.values() {
            self.lanes.remove(*slot);
        }
        self.slots.free.extend(taken.slots.values().copied());
        self.slots.free.sort_unstable_by(|a, b| b.cmp(a));
        debug_assert_eq!(
            misplaced(&self.boxes),
            None,
            "a walk from the root reaches a vacant slot or misses a box"
        );
    }
}
