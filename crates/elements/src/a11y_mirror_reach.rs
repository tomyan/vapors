//! What a take reads (R6): every node a touch reaches, each once. A
//! touch reaches the node, its ancestors up to one reached already
//! (whose own ancestors are in), and the nodes referring by id to it or
//! to an ancestor; an element inserted that is not a node reaches the
//! nearest nodes under it, moved with it while their boxes may have
//! stayed (absolutely positioned); a node named by an id whose element
//! changed, and the nodes focus left and came to, are read as well.

use vapors_dom::NodeId;

use crate::a11y_mirror_index::AxIndex;
use crate::a11y_mirror_state::Touched;
use crate::document::Document;
use crate::node_map::NodeSet;

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
        for &wrapper in &touched.replaced {
            self.ax_wrapped_nodes(wrapper, &mut add);
        }
        for &node in renamed.iter().chain(&touched.renamed) {
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

    /// The nearest nodes under `wrapper` when it is not a node itself:
    /// moved with it, with their boxes kept, so no touch names them. A
    /// wrapper that is a node moves them with its own placement.
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
