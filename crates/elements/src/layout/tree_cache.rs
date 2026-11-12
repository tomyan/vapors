//! Taffy's cache trait for the box tree, and the note of how each box
//! was asked (`LayoutBox::asked`), which `isolated.rs` replays.

use taffy::{Cache, CacheTree, LayoutInput, LayoutOutput, NodeId};

use super::super::asked::Asked;
use super::super::boxes::LayoutBox;
use super::BoxTree;

impl BoxTree<'_> {
    /// Clears the cache, hoisted list and asked inputs of each slot.
    pub(crate) fn clear(&mut self, slots: &[usize]) {
        for &slot in slots {
            let layout_box = &mut self.boxes[slot];
            layout_box.cache.clear();
            layout_box.hoisted.clear();
            layout_box.asked.clear();
            layout_box.asked_overflowed = false;
        }
    }

    /// Notes `asked` among the ways the box at `id` was asked: every
    /// way misses the cache the first time, so the list is the ways
    /// asked since the cache was cleared. A size query answered with
    /// margins misses every time (the cache keeps no such answer), and
    /// a box asked the same way answers the same until its cache is
    /// cleared; one that answers otherwise has its recorded answer
    /// disowned (`Unknown` on replay) and counted. A box asked more
    /// than `ASKED_LIMIT` ways records none until its cache is cleared.
    pub(super) fn note_asked(&mut self, id: NodeId, asked: Asked) {
        let node = self.node_mut(id);
        if node.asked_overflowed {
            return;
        }
        match node.asked.iter_mut().find(|noted| noted.same_way(&asked)) {
            Some(noted) if noted.answered != asked.answered => {
                noted.answered = None;
                self.disowned += 1;
            }
            Some(_) => {}
            None => note_new_way(node, asked),
        }
    }
}

/// The most ways a box records being asked. Each way is about 92
/// bytes; a 400-width drag asked a flex-column row 1,201 ways (3 per
/// width: two size queries and a layout; `isolated_record_tests.rs`),
/// about 110 KB per row. Taffy's cache keeps 10 entries (9 size
/// queries, 1 layout), so the replay of a way beyond those reads
/// `Unknown` in any case; 16 holds five passes of a flex row.
pub(in crate::layout) const ASKED_LIMIT: usize = 16;

/// Notes `asked`, a way `node` was not asked before, or abandons the
/// record at the limit: the ways already asked stay load-bearing for
/// the parent's layout, so a shorter record could not be replayed.
fn note_new_way(node: &mut LayoutBox, asked: Asked) {
    if node.asked.len() < ASKED_LIMIT {
        node.asked.push(asked);
        return;
    }
    node.asked.clear();
    node.asked_overflowed = true;
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
        node.asked_overflowed = false;
    }
}
