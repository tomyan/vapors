//! What a take reads (R6): every node a touch reaches, each once. A
//! touch reaches the node, its ancestors up to one reached already
//! (whose own ancestors are in), and the nodes referring by id to it or
//! to an ancestor; an element inserted that is not a node reaches the
//! nearest nodes under it, moved with it while their boxes may have
//! stayed (absolutely positioned), and so does a row a lane start moved
//! that is not a node, as their bounds are from a node above the row,
//! and the hoisted nodes under such a row, whose bounds are from the
//! viewport;
//! a node named by an id whose element
//! changed, the nodes focus left and came to, and the hoisted nodes in
//! the frame of a scroll container scrolled, are read as well.

use vapors_dom::NodeId;

use crate::a11y_mirror_index::AxIndex;
use crate::a11y_mirror_state::Touched;
use crate::document::Document;
use crate::node_map::NodeSet;
use rustc_hash::FxHashSet;

impl Document {
    /// Every node this take reads, each once, the touched first so that
    /// each comes with its ancestors.
    pub(crate) fn ax_candidates(
        &self,
        touched: &Touched,
        renamed: &[NodeId],
        focus: [Option<NodeId>; 2],
        index: &AxIndex,
    ) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut seen = NodeSet::default();
        let mut add = |node: NodeId| {
            let new = seen.insert(node);
            if new {
                out.push(node);
            }
            new
        };
        for &node in &touched.nodes {
            self.ax_reach(node, index, &mut add);
        }
        for &wrapper in touched.replaced.iter().chain(&touched.started) {
            self.ax_wrapped_nodes(wrapper, &mut add);
        }
        for &node in renamed.iter().chain(&touched.renamed) {
            add(node);
        }
        for node in self.ax_scrolled_hoisted(&touched.scrolled) {
            add(node);
        }
        for node in self.ax_hoisted_under(&touched.started) {
            add(node);
        }
        for node in focus.into_iter().flatten() {
            add(node);
        }
        out
    }

    /// What a touch of `node` reaches: the node, its ancestors up to one
    /// reached already, and the nodes referring by id to each of those.
    fn ax_reach(&self, node: NodeId, index: &AxIndex, add: &mut impl FnMut(NodeId) -> bool) {
        if !add(node) || !self.contains(node) {
            return;
        }
        let mut cursor = Some(node);
        while let Some(reached) = cursor {
            self.ax_referrers(reached, index, add);
            cursor = self.parent(reached).filter(|&parent| add(parent));
        }
    }

    fn ax_referrers(&self, node: NodeId, index: &AxIndex, add: &mut impl FnMut(NodeId) -> bool) {
        let Some(id) = self.tag(node).and_then(|_| self.attribute(node, "id")) else {
            return;
        };
        for &referrer in index.referrers.get(id).into_iter().flatten() {
            add(referrer);
        }
    }

    /// The hoisted nodes whose boxes are in the frame of one of the
    /// scroll containers `scrolled`: a scroll moves them on the page,
    /// and their bounds are from the viewport.
    fn ax_scrolled_hoisted(&self, scrolled: &NodeSet) -> Vec<NodeId> {
        if scrolled.is_empty() || self.ax.hoisted.is_empty() {
            return Vec::new();
        }
        let frames: FxHashSet<usize> = scrolled
            .iter()
            .filter_map(|&container| self.boxes.slot_of(container))
            .collect();
        let in_frame = |&node: &NodeId| self.ax_placed_under(node, &|slot| frames.contains(&slot));
        self.ax.hoisted.iter().copied().filter(in_frame).collect()
    }

    /// The hoisted nodes under any of `rows`, rows a lane start moved:
    /// their bounds are from the viewport, and the row moved on the page
    /// carries the box they are placed against, so neither the row nor a
    /// node between reaches them. O(hoisted nodes times their depth), not
    /// the rows' subtrees.
    fn ax_hoisted_under(&self, rows: &NodeSet) -> Vec<NodeId> {
        if rows.is_empty() || self.ax.hoisted.is_empty() {
            return Vec::new();
        }
        let under_a_row = |&node: &NodeId| {
            let mut cursor = self.parent(node);
            while let Some(above) = cursor {
                if rows.contains(&above) {
                    return true;
                }
                cursor = self.parent(above);
            }
            false
        };
        self.ax
            .hoisted
            .iter()
            .copied()
            .filter(under_a_row)
            .collect()
    }

    /// The nearest nodes under `wrapper` when it is not a node itself:
    /// moved with it, an inserted element or a row a lane start moved,
    /// with their boxes kept, so no touch names them, and their bounds
    /// from a node above `wrapper`. A wrapper that is a node moves them
    /// with its own placement.
    fn ax_wrapped_nodes(&self, wrapper: NodeId, add: &mut impl FnMut(NodeId) -> bool) {
        if self.ax.records.contains_key(&wrapper) || !self.contains(wrapper) {
            return;
        }
        let mut below = self.children(wrapper);
        while let Some(node) = below.pop() {
            if self.ax.records.contains_key(&node) {
                add(node);
            } else {
                below.extend(self.children(node));
            }
        }
    }

    /// Whether `node`, or an element between it and `parent`, was
    /// inserted since the last take.
    pub(crate) fn ax_moved(
        &self,
        node: NodeId,
        parent: Option<NodeId>,
        replaced: &NodeSet,
    ) -> bool {
        let mut cursor = Some(node);
        while let Some(between) = cursor.filter(|&between| Some(between) != parent) {
            if replaced.contains(&between) {
                return true;
            }
            cursor = self.parent(between);
        }
        false
    }
}
