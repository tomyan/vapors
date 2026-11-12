//! A changed owner laid out in isolation (M10, docs/plans/
//! perf-finish-line.md property 1). A rebuilt subtree that changed
//! used to clear its ancestors' caches up to the root, so Taffy laid
//! the root out again and, in it, the container holding the owner,
//! which asked every sibling for its (cached) layout: O(rows) per
//! edit. Now the rebuilt owner is first asked what its parent asked
//! the box it replaced (`LayoutBox::asked`). When every answer is the
//! one the old box gave (a layout's from the old cache, a size query's
//! as the old box recorded it, `asked.rs`), its parent's cached layout
//! still stands, the ancestors keep their caches, and the root's
//! layout is a cache hit. Otherwise the ancestors are cleared as before.
//!
//! Three states, never a guess: `Same` keeps the ancestors; `Differs`
//! (the owner's size, margins or baselines changed, it bubbles an
//! out-of-flow box up to an ancestor, or it is styled otherwise than
//! the box it replaced: its parent reads its style to work out what to
//! ask) and `Unknown` (no old box, no input asked, a layout's input the
//! old cache no longer holds, a size answer disowned or bubbling an
//! out-of-flow box up, a box asked more ways than it records
//! (`tree_cache.rs`), an absolute or fixed owner, which its containing
//! block hoists and places, a restyled owner whose style holds a calc,
//! `keep.rs`, or the root, which no parent asks: `compute_root_layout`
//! places the fixed boxes bubbled up to it outside the cached call,
//! and hoists them onto it only on a cache miss) clear them. Owners
//! settle deepest first: one above
//! another places it when laid out in isolation, which the lower
//! one's `Same`, settled after, would undo by putting its old layout
//! back. Debug builds check the result against a fresh layout
//! (`layout_check.rs`).

use taffy::{
    BlockFormattingContext, Cache, Layout, LayoutOutput, LayoutPartialTree, NodeId, Position,
    RunMode, Style,
};

use super::asked::Answer;
pub(crate) use super::asked::Asked;
use super::boxes::LayoutBox;
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

impl Before {
    /// What `old`, the box replaced, held.
    pub(crate) fn of(old: LayoutBox) -> Self {
        Self {
            cache: old.cache,
            layout: old.layout,
            asked: old.asked,
            style: old.style,
            natural: old.natural,
        }
    }
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
    /// Settles each changed owner before the root is laid out, deepest
    /// first: in isolation where that answers as before, else by
    /// clearing its ancestors' caches and hoisted lists so the root
    /// lays them out again.
    pub(crate) fn settle(&mut self, mut changed: Vec<Changed>) -> Settled {
        changed.sort_by_key(|owner| std::cmp::Reverse(owner.ancestors.len()));
        let mut settled = Settled::default();
        for Changed {
            slot,
            ancestors,
            before,
        } in changed
        {
            let isolation = self.isolate(slot, ancestors.is_empty(), before);
            let count = match isolation {
                Isolation::Same => &mut settled.isolated,
                Isolation::Differs => &mut settled.resized,
                Isolation::Unknown => &mut settled.uncompared,
            };
            *count += 1;
            if !matches!(isolation, Isolation::Same) {
                self.clear(&ancestors);
            }
        }
        settled
    }

    /// Asks the rebuilt box at `slot` each input its parent asked the
    /// box before; `Same` when every answer is the old box's (a
    /// layout's from its cache, a size query's as it recorded it,
    /// `asked.rs`) and none bubbles an out-of-flow box up, with the old
    /// layout restored as the parent would set it. `at_root`: the
    /// owner has no ancestor.
    fn isolate(&mut self, slot: usize, at_root: bool, before: Option<Before>) -> Isolation {
        let Some(mut before) = before else {
            return Isolation::Unknown;
        };
        if let Some(settled) = self.not_comparable(slot, at_root, &before) {
            return settled;
        }
        let id = NodeId::from(slot);
        // Size queries first, PerformLayout last: the order a parent asks.
        before
            .asked
            .sort_by_key(|asked| asked.input.run_mode == RunMode::PerformLayout);
        for asked in &before.asked {
            let Some(old) = Answer::of(asked, &mut before.cache) else {
                return Isolation::Unknown;
            };
            let new = self.ask(id, asked);
            if !new.oof_candidates.is_empty() || !old.is(&new) {
                return Isolation::Differs;
            }
        }
        self.set_unrounded_layout(id, &before.layout);
        self.boxes[slot].asked = before.asked;
        Isolation::Same
    }

    /// The state of the box at `slot` when it cannot be compared with
    /// the box it replaced: `Unknown` at the root, with nothing to
    /// keep above it and `compute_root_layout` hoisting the fixed
    /// boxes only on a cache miss, when never asked, or when hoisted
    /// from above; `Differs` when styled otherwise.
    fn not_comparable(&self, slot: usize, at_root: bool, before: &Before) -> Option<Isolation> {
        if at_root || before.asked.is_empty() || self.hoisted_from_above(slot) {
            return Some(Isolation::Unknown);
        }
        self.styled_otherwise(slot, before)
            .then_some(Isolation::Differs)
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
}

#[cfg(test)]
#[path = "isolated_fallback_tests.rs"]
mod fallback_tests;
#[cfg(test)]
#[path = "isolated_flex_tests.rs"]
mod flex_tests;
#[cfg(all(test, feature = "grid"))]
#[path = "isolated_grid_tests.rs"]
mod grid_tests;
#[cfg(test)]
#[path = "isolated_order_tests.rs"]
mod order_tests;
#[cfg(test)]
#[path = "isolated_record_tests.rs"]
mod record_tests;
#[cfg(test)]
#[path = "isolated_tests.rs"]
mod tests;
