//! Taffy's view of the box tree: its traversal, style, cache and
//! containing-block traits, and the dispatch from a box's display type
//! to Taffy's block, flex and leaf algorithms.

use taffy::{
    BlockContext, Cache, CacheTree, Display, Layout, LayoutInput, LayoutOutput, LayoutPartialTree,
    NodeId, RunMode, Style, TraversePartialTree, TraverseTree, compute_block_layout,
    compute_cached_layout, compute_flexbox_layout, compute_grid_layout, compute_hidden_layout,
    compute_leaf_layout, compute_oof_layout,
};
use vapors_text::FontSystem;

use super::boxes::{Content, LayoutBox};
use super::calcs::Calcs;
use super::measure::measure_text;

/// The boxes, what measuring text needs, and the calcs Taffy may ask for.
pub(crate) struct BoxTree<'f> {
    pub(crate) boxes: Vec<LayoutBox>,
    pub(crate) fonts: &'f mut FontSystem,
    pub(crate) cache: &'f mut super::TextCache,
    pub(crate) scale: f32,
    pub(crate) calcs: &'f Calcs,
    /// Boxes laid out, not read from their cache: what the incremental
    /// tests count.
    pub(crate) laid_out: usize,
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
        self.laid_out += 1;
        let has_children = !self.node(id).children.is_empty();
        match (self.node(id).style.display, has_children) {
            (Display::None, _) => compute_hidden_layout(self, id),
            (Display::Block, true) => compute_block_layout(self, id, inputs, block),
            (Display::Flex, true) => compute_flexbox_layout(self, id, inputs),
            (Display::Grid, true) => compute_grid_layout(self, id, inputs),
            (_, false) => self.leaf(id, inputs),
            (display, true) => unreachable!("{display:?} is refused when styles are read"),
        }
    }

    fn leaf(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let mut output = self.measured_leaf(id, inputs);
        // a replaced element is never empty: margins do not collapse
        // through it, even at 0 x 0, as Chromium lays out an `img`
        // without a `src`
        if self.boxes[usize::from(id)].natural.is_some() {
            output.margins_can_collapse_through = false;
        }
        output
    }

    fn measured_leaf(&mut self, id: NodeId, inputs: LayoutInput) -> LayoutOutput {
        let (fonts, cache, scale, calcs) =
            (&mut *self.fonts, &mut *self.cache, self.scale, self.calcs);
        let node = &self.boxes[usize::from(id)];
        compute_leaf_layout(
            inputs,
            &node.style,
            |handle, basis| calcs.resolve(handle, basis),
            |known, available| match &node.content {
                Content::Text(run) => measure_text((fonts, cache), scale, run, known, available),
                Content::Element(_) => node
                    .natural
                    .map_or(taffy::Size::ZERO, |n| replaced(known, n)),
            },
        )
    }
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
        self.calcs.resolve(calc, basis)
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

/// A replaced box's content size: what layout already knows, else the
/// natural size, a known width or height setting the other by the
/// natural aspect ratio.
fn replaced(known: taffy::Size<Option<f32>>, [width, height]: [f32; 2]) -> taffy::Size<f32> {
    let ratio = |a: f32, b: f32| if b > 0.0 { a / b } else { 0.0 };
    match (known.width, known.height) {
        (Some(w), Some(h)) => taffy::Size {
            width: w,
            height: h,
        },
        (Some(w), None) => taffy::Size {
            width: w,
            height: w * ratio(height, width),
        },
        (None, Some(h)) => taffy::Size {
            width: h * ratio(width, height),
            height: h,
        },
        (None, None) => taffy::Size { width, height },
    }
}
