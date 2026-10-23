//! Taffy's container traits for the box tree: every box answers with its
//! own style, and a block child keeps its block formatting context.

use taffy::{
    BlockContext, LayoutBlockContainer, LayoutContainingBlock, LayoutFlexboxContainer,
    LayoutGridContainer, LayoutInput, LayoutOutput, NodeId, Style,
};

use super::BoxTree;

impl LayoutBlockContainer for BoxTree<'_> {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }

    fn get_block_child_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }

    fn compute_block_child_layout(
        &mut self,
        id: NodeId,
        inputs: LayoutInput,
        block: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        self.lay_out(id, inputs, block)
    }
}

impl LayoutFlexboxContainer for BoxTree<'_> {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }

    fn get_flexbox_child_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }
}

impl LayoutGridContainer for BoxTree<'_> {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }

    fn get_grid_child_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }
}

/// Absolute and fixed boxes are hoisted to their containing block,
/// whose list `place` reads to position them.
impl LayoutContainingBlock for BoxTree<'_> {
    type OofItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_oof_item_style(&self, id: NodeId) -> &Style {
        &self.boxes[usize::from(id)].style
    }

    fn clear_hoisted_children(&mut self, id: NodeId) {
        self.boxes[usize::from(id)].hoisted.clear();
    }

    fn add_hoisted_children(&mut self, id: NodeId, hoisted: &[NodeId]) {
        let list = &mut self.boxes[usize::from(id)].hoisted;
        list.extend(hoisted.iter().map(|child| usize::from(*child)));
    }
}
