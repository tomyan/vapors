//! Box slots (IU3): a box keeps its index in the box vector, which is
//! also Taffy's `NodeId`, across updates while its key exists, so the
//! caches Taffy keeps, and the out-of-flow boxes they name, stay valid.
//! A slot whose box is gone holds `Content::Vacant`, which no box walk
//! from the root reaches, until a new key takes it from the free list.

use std::collections::HashMap;

use vapors_dom::NodeId;

use super::boxes::{Content, LayoutBox};

/// Which box a box is across updates: an element's, or the text box
/// at a position among its parent's text boxes.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Key {
    Element(NodeId),
    Text(NodeId, usize),
}

/// The slot each key holds, each slot's key, and the vacant slots.
#[derive(Default)]
pub(crate) struct Slots {
    of: HashMap<Key, usize>,
    keys: Vec<Option<Key>>,
    /// Vacant slots, highest first, so the lowest is taken first.
    free: Vec<usize>,
}

/// Boxes in their slots, with the slots they were placed by.
#[derive(Default)]
pub(crate) struct Placed {
    pub(crate) boxes: Vec<LayoutBox>,
    pub(crate) slots: Slots,
}

impl Slots {
    /// The slot `key`'s box holds.
    pub(crate) fn of(&self, key: Key) -> Option<usize> {
        self.of.get(&key).copied()
    }

    /// The key of the box in `slot`; none for a vacant slot.
    pub(crate) fn key(&self, slot: usize) -> Option<Key> {
        self.keys.get(slot).copied().flatten()
    }

    /// Places `built`, boxes in pre-order with the root first, into
    /// slots: a key these slots hold keeps its slot, a new key takes the
    /// lowest vacant slot or a new one at the end, and a slot whose key
    /// is gone becomes vacant. A root other than slot 0's starts afresh.
    pub(crate) fn place(&self, mut built: Vec<LayoutBox>) -> Placed {
        let keys = keys(&built);
        let Some(&root) = keys.first() else {
            return Placed::default();
        };
        if !self.keys.is_empty() && self.of(root) != Some(0) {
            return Self::default().place(built);
        }
        let (to, slots) = self.assign(&keys);
        for layout_box in &mut built {
            for child in &mut layout_box.children {
                *child = to[*child];
            }
        }
        let boxes = arrange(built, &to, slots.keys.len());
        debug_assert_eq!(
            misplaced(&boxes),
            None,
            "a walk from the root reaches a vacant slot or misses a box"
        );
        Placed { boxes, slots }
    }

    /// Each new key's slot, by its pre-order index, and the slots after.
    fn assign(&self, keys: &[Key]) -> (Vec<usize>, Slots) {
        let kept: Vec<Option<usize>> = keys.iter().map(|key| self.of(*key)).collect();
        let mut free = self.freed(&kept);
        let mut len = self.keys.len();
        let to: Vec<usize> = kept
            .into_iter()
            .map(|slot| {
                slot.or_else(|| free.pop())
                    .unwrap_or_else(|| next(&mut len))
            })
            .collect();
        let slots = Self::holding(keys, &to, len, free);
        (to, slots)
    }

    /// `len` slots, `keys` in the slots `to` gives, `free` vacant.
    fn holding(keys: &[Key], to: &[usize], len: usize, free: Vec<usize>) -> Self {
        let mut slots = Self {
            of: HashMap::with_capacity(keys.len()),
            keys: vec![None; len],
            free,
        };
        for (key, &slot) in keys.iter().zip(to) {
            let earlier = slots.of.insert(*key, slot);
            assert!(earlier.is_none(), "box key {key:?} placed twice");
            let other = slots.keys[slot].replace(*key);
            assert!(
                other.is_none(),
                "slot {slot} given to {key:?} and {other:?}"
            );
        }
        slots
    }

    /// The slots free for new keys: those vacant already and those
    /// whose key no box in `kept` holds, highest first.
    fn freed(&self, kept: &[Option<usize>]) -> Vec<usize> {
        let mut held = vec![false; self.keys.len()];
        for &slot in kept.iter().flatten() {
            held[slot] = true;
        }
        let gone = (0..self.keys.len()).filter(|&slot| self.keys[slot].is_some() && !held[slot]);
        let mut free: Vec<usize> = self.free.iter().copied().chain(gone).collect();
        free.sort_unstable_by(|a, b| b.cmp(a));
        free
    }
}

/// `*len`, which then counts one more slot.
fn next(len: &mut usize) -> usize {
    *len += 1;
    *len - 1
}

/// `built` moved into the slots `to` gives, `len` in all, the slots
/// nothing takes holding vacant boxes.
fn arrange(mut built: Vec<LayoutBox>, to: &[usize], len: usize) -> Vec<LayoutBox> {
    if len == built.len() && to.iter().enumerate().all(|(index, &slot)| index == slot) {
        return built;
    }
    let mut taken = vec![false; len];
    for &slot in to {
        taken[slot] = true;
    }
    let mut destination = to.to_vec();
    destination.extend((0..len).filter(|&slot| !taken[slot]));
    built.resize_with(len, LayoutBox::vacant);
    permute(&mut built, destination);
    built
}

/// Moves each box to its `destination`, a permutation, by swaps.
fn permute(boxes: &mut [LayoutBox], mut destination: Vec<usize>) {
    for index in 0..boxes.len() {
        while destination[index] != index {
            let slot = destination[index];
            boxes.swap(index, slot);
            destination.swap(index, slot);
        }
    }
}

/// The first slot a walk from the root gets wrong: a vacant slot it
/// reaches, or a box it misses; none when it reaches exactly the boxes.
fn misplaced(boxes: &[LayoutBox]) -> Option<usize> {
    let seen = reached(boxes);
    (0..boxes.len()).find(|&slot| seen[slot] == matches!(boxes[slot].content, Content::Vacant))
}

/// The slots a walk from the root reaches.
fn reached(boxes: &[LayoutBox]) -> Vec<bool> {
    let mut seen = vec![false; boxes.len()];
    let mut stack = vec![0];
    while let Some(slot) = stack.pop() {
        seen[slot] = true;
        stack.extend(&boxes[slot].children);
    }
    seen
}

/// Every box's key, by index, in boxes built in pre-order.
fn keys(boxes: &[LayoutBox]) -> Vec<Key> {
    let mut keys = vec![Key::Text(NodeId(0), 0); boxes.len()];
    for (index, layout_box) in boxes.iter().enumerate() {
        if let Content::Element(node) = layout_box.content {
            keys[index] = Key::Element(node);
            let mut texts = 0;
            for &child in &layout_box.children {
                if matches!(boxes[child].content, Content::Text(_)) {
                    keys[child] = Key::Text(node, texts);
                    texts += 1;
                }
            }
        }
    }
    keys
}

#[cfg(test)]
#[path = "slots_tests.rs"]
pub(super) mod tests;
