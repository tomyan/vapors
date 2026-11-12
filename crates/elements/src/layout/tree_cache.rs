//! Taffy's cache trait for the box tree, and the note of how each box
//! was asked (`LayoutBox::asked`), which `isolated.rs` replays.

use taffy::{Cache, CacheTree, LayoutInput, LayoutOutput, NodeId};

use super::super::isolated::Asked;
use super::BoxTree;

impl BoxTree<'_> {
    /// Notes `asked` among the ways the box at `id` was asked: every
    /// way misses the cache the first time, so the list is the ways
    /// asked since the cache was cleared.
    pub(super) fn note_asked(&mut self, id: NodeId, asked: Asked) {
        let node = self.node_mut(id);
        if !node.asked.contains(&asked) {
            node.asked.push(asked);
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
