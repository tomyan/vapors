//! Taffy's cache trait for the box tree, and the note of how each box
//! was asked (`LayoutBox::asked`), which `isolated.rs` replays.

use taffy::{Cache, CacheTree, LayoutInput, LayoutOutput, NodeId};

use super::super::asked::Asked;
use super::BoxTree;

impl BoxTree<'_> {
    /// Clears the cache, hoisted list and asked inputs of each slot.
    pub(crate) fn clear(&mut self, slots: &[usize]) {
        for &slot in slots {
            let layout_box = &mut self.boxes[slot];
            layout_box.cache.clear();
            layout_box.hoisted.clear();
            layout_box.asked.clear();
        }
    }

    /// Notes `asked` among the ways the box at `id` was asked: every
    /// way misses the cache the first time, so the list is the ways
    /// asked since the cache was cleared. A size query answered with
    /// margins misses every time (the cache keeps no such answer), and
    /// a box asked the same way answers the same until its cache is
    /// cleared, which a debug build checks.
    pub(super) fn note_asked(&mut self, id: NodeId, asked: Asked) {
        let node = self.node_mut(id);
        match node.asked.iter().find(|noted| noted.same_way(&asked)) {
            None => node.asked.push(asked),
            Some(noted) => debug_assert_eq!(
                noted.answered,
                asked.answered,
                "the box in slot {} answered the same size query otherwise since its cache was \
                 cleared, so a layout it was kept in rests on a stale answer; \
                 `cargo test -p vapors-elements` reproduces it",
                usize::from(id)
            ),
        }
    }
}

impl CacheTree for BoxTree<'_> {
    fn cache_get(&mut self, id: NodeId, inputs: &LayoutInput) -> Option<LayoutOutput> {
        self.node_mut(id).cache.get(inputs)
    }

    fn cache_store(&mut self, id: NodeId, inputs: &LayoutInput, output: LayoutOutput) {
        self.node_mut(id).cache.store(inputs, output);
    }

    fn cache_clear(&mut self, id: NodeId) {
        let node = self.node_mut(id);
        node.cache = Cache::new();
        node.asked.clear();
    }
}
