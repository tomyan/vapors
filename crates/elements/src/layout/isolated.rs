//! A changed owner laid out in isolation (M10, docs/plans/
//! perf-finish-line.md property 1). A rebuilt subtree that changed
//! used to clear its ancestors' caches up to the root, so Taffy laid
//! the root out again and, in it, the container holding the owner,
//! which asked every sibling for its (cached) layout: O(rows) per
//! edit. Now the rebuilt owner is first asked what its parent asked
//! the box it replaced (`LayoutBox::asked`). When every answer is the
//! one the old cache holds, its parent's cached layout still stands,
//! the ancestors keep their caches, and the root's layout is a cache
//! hit. Otherwise the ancestors are cleared as before.
//!
//! Three states, never a guess: `Same` keeps the ancestors; `Differs`
//! (the owner's size, margins or baselines changed, or it bubbles an
//! out-of-flow box up to an ancestor) and `Unknown` (no old box, no
//! input asked, an input the old cache no longer holds, an absolute
//! or fixed owner, which its containing block hoists and places, or a
//! restyled owner whose style holds a calc, `keep.rs`) clear them. An
//! owner styled otherwise than the box it replaced differs too: its
//! parent reads its style to work out what to ask.
//! Debug builds check the result against a fresh layout
//! (`layout_check.rs`).

use taffy::{
    BlockFormattingContext, Cache, Layout, LayoutInput, LayoutOutput, LayoutPartialTree, NodeId,
    Position, Style,
};

use super::tree::BoxTree;

/// A rebuilt owner whose subtree changed, and what settling it needs.
#[derive(Debug)]
pub(crate) struct Changed {
    pub(crate) slot: usize,
    /// The slots of the element boxes above it, nearest first.
    pub(crate) ancestors: Vec<usize>,
    /// What the box it replaced held; none when the slot held no box
    /// of its key, which leaves nothing to compare.
    pub(crate) before: Option<Before>,
}

/// What the replaced box held after its last layout. Its style and
/// natural size too: a parent reads a child's style to work out what
/// to ask it, so a box styled otherwise is asked otherwise, whatever
/// it answers to the old questions.
#[derive(Debug)]
pub(crate) struct Before {
    pub(crate) cache: Cache,
    pub(crate) layout: Layout,
    pub(crate) asked: Vec<Asked>,
    pub(crate) style: Style,
    pub(crate) natural: Option<[f32; 2]>,
}

/// One way a box was asked for its layout: the input, and whether in
/// its parent's block formatting context (`BoxTree::lay_out` given a
/// `BlockContext`), which a block answers differently from a root.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Asked {
    pub(crate) input: LayoutInput,
    pub(crate) in_block: bool,
}

/// How the changed owners were settled, by state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Settled {
    /// Laid out in isolation with the answers the old box gave: their
    /// ancestors keep their caches.
    pub(crate) isolated: usize,
    /// Answered differently: their ancestors are laid out again.
    pub(crate) resized: usize,
    /// Nothing to compare against: their ancestors are laid out again.
    pub(crate) uncompared: usize,
}

enum Isolation {
    Same,
    Differs,
    Unknown,
}

impl BoxTree<'_> {
    /// Settles each changed owner before the root is laid out: in
    /// isolation where that answers as before, else by clearing its
    /// ancestors' caches and hoisted lists so the root lays them out
    /// again.
    pub(crate) fn settle(&mut self, changed: Vec<Changed>) -> Settled {
        let mut settled = Settled::default();
        for owner in changed {
            let isolation = self.isolate(owner.slot, owner.before);
            let count = match isolation {
                Isolation::Same => &mut settled.isolated,
                Isolation::Differs => &mut settled.resized,
                Isolation::Unknown => &mut settled.uncompared,
            };
            *count += 1;
            if !matches!(isolation, Isolation::Same) {
                self.clear(&owner.ancestors);
            }
        }
        settled
    }

    /// Asks the rebuilt box at `slot` each input its parent asked the
    /// box before; `Same` when every answer is the old cache's and none
    /// bubbles an out-of-flow box up, with the old layout restored as
    /// the parent would set it.
    fn isolate(&mut self, slot: usize, before: Option<Before>) -> Isolation {
        let Some(mut before) = before else {
            return Isolation::Unknown;
        };
        if before.asked.is_empty() || self.hoisted_from_above(slot) {
            return Isolation::Unknown;
        }
        if self.styled_otherwise(slot, &before) {
            return Isolation::Differs;
        }
        let id = NodeId::from(slot);
        for asked in &before.asked {
            let Some(old) = before.cache.get(&asked.input) else {
                return Isolation::Unknown;
            };
            let new = self.ask(id, asked);
            if new != old || !new.oof_candidates.is_empty() {
                return Isolation::Differs;
            }
        }
        self.set_unrounded_layout(id, &before.layout);
        self.boxes[slot].asked = before.asked;
        Isolation::Same
    }

    /// Whether the box at `slot` has another style or natural size
    /// than the box it replaced, so its parent would ask it otherwise.
    fn styled_otherwise(&self, slot: usize, before: &Before) -> bool {
        let layout_box = &self.boxes[slot];
        layout_box.style != before.style || layout_box.natural != before.natural
    }

    /// Whether the box at `slot` is placed by a containing block above
    /// it, which hoists it when laid out (`tree_containers.rs`): laid
    /// out alone, it would be in no hoisted list layout wrote.
    fn hoisted_from_above(&self, slot: usize) -> bool {
        matches!(
            self.boxes[slot].style.position,
            Position::Absolute | Position::Fixed
        )
    }

    /// Asks the box at `id` as its parent did: a block child in its
    /// parent's block formatting context is laid out in one, not as the
    /// root of its own, which would not collapse margins through it.
    /// The context's offsets place floats, which vapors does not lay
    /// out (taffy's `float_layout` feature is off), so a fresh one
    /// answers as the parent's did; floats would need the parent's.
    fn ask(&mut self, id: NodeId, asked: &Asked) -> LayoutOutput {
        if !asked.in_block {
            return self.lay_out(id, asked.input, None);
        }
        let mut formatting = BlockFormattingContext::new();
        let mut root = formatting.root_block_context();
        let mut context = root.sub_context(0.0, [0.0, 0.0]);
        self.lay_out(id, asked.input, Some(&mut context))
    }

    /// Clears the cache, hoisted list and asked inputs of each slot.
    pub(crate) fn clear(&mut self, slots: &[usize]) {
        for &slot in slots {
            let layout_box = &mut self.boxes[slot];
            layout_box.cache.clear();
            layout_box.hoisted.clear();
            layout_box.asked.clear();
        }
    }
}

#[cfg(test)]
#[path = "isolated_fallback_tests.rs"]
mod fallback_tests;
#[cfg(test)]
#[path = "isolated_tests.rs"]
mod tests;
