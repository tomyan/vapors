//! One child of a block lane laid out as taffy 0.15.1's block algorithm
//! lays an in-flow, non-floated item out (`compute/block.rs`,
//! `perform_final_layout_on_in_flow_children` 1158-1167, 1293-1304,
//! 1383-1448, 1462-1520, 1567-1601, 1694-1752), one item at a time: the
//! leaf for the lane's tree and the child's layout, with its position
//! along the axis left to the lane, which derives it from the tree. The
//! child's style is `lane_block_item.rs`, the cross-axis placement
//! `lane_block_child_place.rs`; `lane_block.rs` carries Taffy's MIT
//! notice.

use taffy::util::MaybeMath;
use taffy::{
    AlignItemsKeyword, AvailableSpace, BlockFormattingContext, Layout, LayoutInput, LayoutOutput,
    Line, NodeId, Overflow, Point, Rect, RequestedAxis, RunMode, Size, SizingMode,
};

use super::lane_block::Container;
use super::lane_block_child_place::{Placing, contribution, placed_x, relative_offset};
use super::lane_block_item::Item;
use super::lane_block_summary::{BlockChild, Margins};
use super::lane_bounds::Bounds;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// A child laid out: its leaf, and its layout whose `location.y` holds
/// only its relative offset, to which the lane adds its start.
pub(crate) struct LaidChild {
    pub(crate) leaf: BlockChild,
    pub(crate) layout: Layout,
}

impl BoxTree<'_> {
    /// Lays the child at `id` out in `c`, as one item of its run.
    ///
    /// # Errors
    /// The child is not one a lane lays out (`LaneDecline`).
    pub(crate) fn lane_child(
        &mut self,
        id: NodeId,
        c: &Container,
    ) -> Result<LaidChild, LaneDecline> {
        let item = self.item(id, c)?;
        let non_auto_margin = item.margin.map(|margin| margin.unwrap_or(0.0));
        let stretch_width = (c.inner_width - non_auto_margin.horizontal_axis_sum()).max(0.0);
        let known = known_dimensions(&item, stretch_width);
        let inputs = child_inputs(c, &item, known, stretch_width);
        let output = self.child_output(id, c, &item, (inputs, non_auto_margin, stretch_width));
        if !output.oof_candidates.is_empty() {
            return Err(LaneDecline::HoistedThrough);
        }
        let calcs = self.calcs;
        let inset_offset = relative_offset(&item, c, |val, basis| calcs.resolve(val, basis));
        let margin = auto_margins(item.margin, (stretch_width - output.size.width).max(0.0));
        let placing = Placing {
            size: output.size,
            location: Point {
                x: placed_x(c, &item, (output.size.width, margin), inset_offset.x),
                y: inset_offset.y,
            },
        };
        Ok(LaidChild {
            leaf: leaf(c, &item, &output, &placing),
            layout: layout(&item, &output, &placing, margin),
        })
    }

    /// The child's layout as its parent would ask it (1462-1492): in
    /// a block context of its own under the container's when in the
    /// same formatting context, else as a root of its own.
    fn child_output(
        &mut self,
        id: NodeId,
        c: &Container,
        item: &Item,
        (inputs, non_auto_margin, stretch_width): (LayoutInput, Rect<f32>, f32),
    ) -> LayoutOutput {
        if !item.same_bfc {
            return self.lay_out(id, inputs, None);
        }
        let width = inputs
            .known_dimensions
            .width
            .unwrap_or(stretch_width)
            .max(item.padding_border_sum.width);
        let inset_left = non_auto_margin.left + c.content_box_inset.left;
        let insets = [inset_left, c.outer_width - width - inset_left];
        let mut formatting = BlockFormattingContext::new();
        let mut root = formatting.root_block_context();
        let mut context = root.sub_context(0.0, insets);
        self.lay_out(id, inputs, Some(&mut context))
    }
}

/// What the child is asked with (1383-1435): a stretched or compressed
/// width, clamped; no sizing keyword, which `item` refused.
fn known_dimensions(item: &Item, stretch_width: f32) -> Size<Option<f32>> {
    if item.compressible_replaced && item.justify_self.keyword != AlignItemsKeyword::Stretch {
        return Size::NONE;
    }
    item.size
        .map_width(|width| {
            Some(
                width
                    .unwrap_or(stretch_width)
                    .maybe_clamp(item.min.width, item.max.width),
            )
        })
        .maybe_clamp(item.min, item.max)
}

/// The child's inputs (1439-1448): a layout whatever the container was
/// asked, as a size query would leave the child's overflow out of its
/// leaf, and the leaf is kept for the container's layout too.
fn child_inputs(
    c: &Container,
    item: &Item,
    known_dimensions: Size<Option<f32>>,
    stretch_width: f32,
) -> LayoutInput {
    LayoutInput {
        run_mode: RunMode::PerformLayout,
        sizing_mode: SizingMode::InherentSize,
        axis: RequestedAxis::Both,
        known_dimensions,
        known_dimensions_are_definite: Size {
            width: true,
            height: true,
        },
        parent_size: c.parent_size,
        available_space: Size {
            width: AvailableSpace::Definite(stretch_width),
            height: AvailableSpace::MaxContent,
        },
        vertical_margins_are_collapsible: if item.same_bfc {
            Line::TRUE
        } else {
            Line::FALSE
        },
    }
}

/// Auto horizontal margins share the free space; vertical auto margins
/// are 0 (1499-1516).
fn auto_margins(margin: Rect<Option<f32>>, free_x: f32) -> Rect<f32> {
    let auto_count = u8::from(margin.left.is_none()) + u8::from(margin.right.is_none());
    let auto = if auto_count > 0 {
        free_x / f32::from(auto_count)
    } else {
        0.0
    };
    Rect {
        left: margin.left.unwrap_or(auto),
        right: margin.right.unwrap_or(auto),
        top: margin.top.unwrap_or(0.0),
        bottom: margin.bottom.unwrap_or(0.0),
    }
}

/// The child's layout (1722-1732). Taffy numbers children by their
/// place in the list; nothing in vapors reads it, and a lane child's
/// place would cost a rank query, so every lane child is 0.
fn layout(item: &Item, output: &LayoutOutput, placing: &Placing, margin: Rect<f32>) -> Layout {
    let scrollbar = |axis: Overflow| (axis == Overflow::Scroll).then_some(item.scrollbar_width);
    Layout {
        order: 0,
        size: output.size,
        scrollable_overflow_rect: output.scrollable_overflow_rect,
        scrollbar_size: Size {
            width: scrollbar(item.overflow.y).unwrap_or(0.0),
            height: scrollbar(item.overflow.x).unwrap_or(0.0),
        },
        location: placing.location,
        padding: item.padding,
        border: item.border,
        margin,
    }
}

/// The child's leaf: its margin sets joined with its own margins
/// (1496-1497), its height, whether it collapses through, and what it
/// reaches from its border-box start: its baseline (1694-1700) and its
/// overflow contribution (1734-1752).
fn leaf(c: &Container, item: &Item, output: &LayoutOutput, placing: &Placing) -> BlockChild {
    let top = output
        .top_margin
        .collapse_with_margin(item.margin.top.unwrap_or(0.0));
    let bottom = output
        .bottom_margin
        .collapse_with_margin(item.margin.bottom.unwrap_or(0.0));
    let baseline =
        child_first_baseline(output.baselines.first, output.size.height, item.overflow.y);
    let (along, across) = contribution(c, item, output, placing);
    BlockChild {
        top: Margins::set(top),
        bottom: Margins::set(bottom),
        height: f64::from(output.size.height),
        collapses_through: output.margins_can_collapse_through,
        bounds: Bounds {
            low: along[0],
            reach: along[1],
            end: along[1],
            across,
            across_end: across[1],
            baseline: baseline.map(f64::from),
            cross: [0.0; 2],
        },
    }
}

/// A scroll container's baseline is clamped into its box
/// (taffy `compute/common/alignment.rs` 12-18).
fn child_first_baseline(first: Option<f32>, height: f32, overflow_y: Overflow) -> Option<f32> {
    if overflow_y.is_scroll_container() {
        Some(first.unwrap_or(height).min(height).max(0.0))
    } else {
        first
    }
}
