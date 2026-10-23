//! Taffy's view of the box tree: its traversal, style, cache and
//! containing-block traits, and the dispatch from a box's display type
//! to Taffy's block, flex and leaf algorithms.

use taffy::{
    BlockContext, Cache, CacheTree, Display, Layout, LayoutInput, LayoutOutput, LayoutPartialTree,
    NodeId, RunMode, Style, TraversePartialTree, TraverseTree, compute_block_layout,
    compute_cached_layout, compute_flexbox_layout, compute_hidden_layout, compute_leaf_layout,
    compute_oof_layout,
};
use vapors_text::FontSystem;

use super::boxes::{Content, LayoutBox};
use super::measure::measure_text;

/// The boxes, and what measuring text needs.
pub(crate) struct BoxTree<'f> {
    pub(crate) boxes: Vec<LayoutBox>,
    pub(crate) fonts: &'f mut FontSystem,
    pub(crate) scale: f32,
}

impl BoxTree<'_> {
    fn node(&self, id: NodeId) -> &LayoutBox {
        &self.boxes[usize::from(id)]
    }

    fn node_mut(&mut self, id: NodeId) -> &mut LayoutBox {
        &mut self.boxes[usize::from(id)]
    }

    fn lay_out(
        &mut self,
        id: NodeId,
        inputs: LayoutInput,
        block: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return compute_hidden_layout(self, id);
        }
        compute_cached_layout(self, id, inputs, |tree, id, inputs| {
            let mut output = tree.dispatch(id, inputs, block);
            if inputs.run_mode == RunMode::PerformLayout {
                compute_oof_layout(tree, id, &mut output);
            }
            output
        })
    }

    fn dispatch(
        &mut self,
        id: NodeId,
        inputs: LayoutInput,
        block: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        let has_children = !self.node(id).children.is_empty();
        match (self.node(id).style.display, has_children) {
            (Display::None, _) => compute_hidden_layout(self, id),
            (Display::Block, true) => compute_block_layout(self, id, inputs, block),
            (Display::Flex, true) => compute_flexbox_layout(self, id, inputs),
            (_, false) => self.leaf(id, inputs),
            (display, true) => unreachable!("{display:?} is refused when styles are read"),
        }
    }

    fn leaf(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let index = usize::from(id);
        let (fonts, scale) = (&mut *self.fonts, self.scale);
        let node = &self.boxes[index];
        compute_leaf_layout(
            inputs,
            &node.style,
            refuse_calc,
            |known, available| match &node.content {
                Content::Text { text, style } => {
                    measure_text(fonts, scale, text, style, known, available)
                }
                Content::Element(_) => taffy::Size::ZERO,
            },
        )
    }
}

/// `calc()` with a % term is refused when styles are read (F2c S4), so
/// Taffy never holds one.
fn refuse_calc(_calc: *const (), _basis: f32) -> f32 {
    unreachable!("calc() values are refused before layout until F2c S4")
}

/// The children of a box, as Taffy ids.
pub(crate) struct Children<'a>(std::slice::Iter<'a, usize>);

impl Iterator for Children<'_> {
    type Item = NodeId;
    fn next(&mut self) -> Option<NodeId> {
        self.0.next().copied().map(NodeId::from)
    }
}

impl TraversePartialTree for BoxTree<'_> {
    type ChildIter<'a>
        = Children<'a>
    where
        Self: 'a;

    fn child_ids(&self, id: NodeId) -> Children<'_> {
        Children(self.node(id).children.iter())
    }

    fn child_count(&self, id: NodeId) -> usize {
        self.node(id).children.len()
    }

    fn get_child_id(&self, id: NodeId, index: usize) -> NodeId {
        NodeId::from(self.node(id).children[index])
    }
}

impl TraverseTree for BoxTree<'_> {}

impl LayoutPartialTree for BoxTree<'_> {
    type CustomIdent = String;
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_core_container_style(&self, id: NodeId) -> &Style {
        &self.node(id).style
    }

    fn resolve_calc_value(&self, calc: *const (), basis: f32) -> f32 {
        refuse_calc(calc, basis)
    }

    fn set_unrounded_layout(&mut self, id: NodeId, layout: &Layout) {
        self.node_mut(id).layout = *layout;
    }

    fn compute_child_layout(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        self.lay_out(id, inputs, None)
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
        self.node_mut(id).cache = Cache::new();
    }
}

#[path = "tree_containers.rs"]
mod containers;
