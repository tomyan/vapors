//! One item of a lane column laid out as taffy 0.15.1's flex algorithm
//! lays out an item of a single-line column of known width
//! (`compute/flexbox.rs`, `determine_hypothetical_cross_size`
//! 1945-2044, `determine_used_cross_size` 2228-2281,
//! `resolve_cross_axis_auto_margins` 2355-2408,
//! `align_flex_items_along_cross_axis` 2410-2502, `calculate_flex_item`
//! 2557-2678): the leaf for the lane's tree and the item's layout, with
//! its position along the column left to the lane. Its height is its
//! hypothetical height: no item grows, and a column whose items would
//! shrink is declined from the fold (`lane_flex_answer.rs`).
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeMath;
use taffy::{
    AvailableSpace, Layout, LayoutInput, LayoutOutput, Line, NodeId, Point, Rect, RequestedAxis,
    RunMode, Size, SizingMode,
};

use super::lane_flex::Column;
use super::lane_flex_base::{Base, transferred};
use super::lane_flex_item::FlexChild;
use super::lane_flex_leaf::{item_layout, leaf};
use super::lane_flex_place::{column_width, cross_place, line_cross};
use super::lane_flex_summary::FlexItem;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// A lane column item laid out: its leaf, and its layout with only its
/// relative offset along the column.
pub(crate) struct LaidItem {
    pub(crate) leaf: FlexItem,
    pub(crate) layout: Layout,
}

/// The item's sizes as laid out: inner, and its margins once its auto
/// margins across took the free space.
pub(super) struct Target {
    pub(super) size: Size<f32>,
    pub(super) margin: Rect<f32>,
}

impl BoxTree<'_> {
    /// Lays the item at `id` out in `column`.
    ///
    /// # Errors
    /// The item is not one a lane lays out (`LaneDecline`).
    pub(crate) fn lane_item(
        &mut self,
        id: NodeId,
        column: &Column,
    ) -> Result<LaidItem, LaneDecline> {
        let child = self.flex_child(id, column)?;
        let base = self.flex_base(id, column, &child)?;
        let [basis, content] = self.flex_contributions(id, column, &child, &base)?;
        let width = self.used_width(id, column, &child, &base);
        let (margin, offset_x) = cross_place(column, &child, width);
        let target = Target {
            size: Size {
                width,
                height: base.hypothetical_inner,
            },
            margin,
        };
        let output = self.item_output(id, column, (&child, &base), &target);
        if !output.oof_candidates.is_empty() {
            return Err(LaneDecline::HoistedThrough);
        }
        let location = Point {
            x: column_x(column) + offset_x + margin.left + child.relative_inset.x,
            y: child.relative_inset.y,
        };
        Ok(LaidItem {
            leaf: leaf(
                column,
                (&child, &base),
                &output,
                (location, margin),
                [basis, content],
            ),
            layout: item_layout(&child, &output, location, margin),
        })
    }

    /// The item's width (2228-2281): stretched across the line, else
    /// its hypothetical width.
    fn used_width(&mut self, id: NodeId, column: &Column, child: &FlexChild, base: &Base) -> f32 {
        if child.is_stretched() {
            return (line_cross(column) - child.margin.horizontal_axis_sum())
                .max(0.0)
                .maybe_clamp(child.min_size.width, child.max_size.width);
        }
        self.hypothetical_width(id, column, child, base)
    }

    /// The item's width when not stretched (1954-2043): its own, else
    /// measured with its height known, clamped, at least its padding
    /// and border.
    fn hypothetical_width(
        &mut self,
        id: NodeId,
        column: &Column,
        child: &FlexChild,
        base: &Base,
    ) -> f32 {
        let (min, max) = transferred(child);
        let padding_border = (child.padding + child.border).horizontal_axis_sum();
        if let Some(width) = child.size.width.maybe_clamp(min.width, max.width) {
            return width.max(padding_border);
        }
        let available_width = column
            .available_space
            .width
            .map_definite_value(|val| (val - child.margin.horizontal_axis_sum()).max(0.0))
            .maybe_clamp(min.width, max.width)
            .maybe_max(padding_border);
        let inputs = width_query(column, (child, base), available_width);
        let measured = self.lay_out(id, inputs, None).size.width;
        measured
            .maybe_clamp(min.width, max.width)
            .max(padding_border)
    }

    /// The item laid out at its target size (2572-2585).
    fn item_output(
        &mut self,
        id: NodeId,
        column: &Column,
        (child, base): (&FlexChild, &Base),
        target: &Target,
    ) -> LayoutOutput {
        let available_width = (column_width(column) - target.margin.horizontal_axis_sum()).max(0.0);
        let inputs = LayoutInput {
            run_mode: RunMode::PerformLayout,
            sizing_mode: SizingMode::ContentSize,
            axis: RequestedAxis::Both,
            known_dimensions: target.size.map(Some),
            known_dimensions_are_definite: definiteness(column, child, base),
            parent_size: column.node_inner_size,
            available_space: Size {
                width: AvailableSpace::Definite(available_width),
                height: available_along(column, base, target.margin),
            },
            vertical_margins_are_collapsible: Line::FALSE,
        };
        self.lay_out(id, inputs, None)
    }
}

/// How the item is asked for its width, its height known (2014-2033).
fn width_query(
    column: &Column,
    (child, base): (&FlexChild, &Base),
    available_width: AvailableSpace,
) -> LayoutInput {
    LayoutInput {
        run_mode: RunMode::ComputeSize,
        sizing_mode: SizingMode::ContentSize,
        axis: RequestedAxis::Horizontal,
        known_dimensions: Size {
            width: None,
            height: Some(base.hypothetical_inner),
        },
        known_dimensions_are_definite: definiteness(column, child, base),
        parent_size: column.node_inner_size,
        available_space: Size {
            width: available_width,
            height: available_along(column, base, child.margin),
        },
        vertical_margins_are_collapsible: Line::FALSE,
    }
}

/// The space along the column an item is asked with: the column's
/// height less the item's margins (1970-1971, 2582). Where that height
/// is the items' total, which one item does not read, the item's own
/// hypothetical height stands in: the item's height is known in every
/// ask that passes it, so neither Taffy's cache nor the item's layout
/// reads it, and the item answers as it would to Taffy.
fn available_along(column: &Column, base: &Base, margin: Rect<f32>) -> AvailableSpace {
    let outer = column
        .node_outer_size
        .height
        .unwrap_or(base.hypothetical_outer);
    AvailableSpace::Definite((outer - margin.vertical_axis_sum()).max(0.0))
}

/// Which of the item's known sizes are definite
/// (`item_known_dimension_definiteness` 1397-1418).
fn definiteness(column: &Column, child: &FlexChild, base: &Base) -> Size<bool> {
    Size {
        width: child.is_stretched()
            || child.size.width.is_some()
            || column.cross_axis_available_space_is_definite(),
        height: column.has_definite_main_size() || base.flex_basis_is_definite,
    }
}

/// Where the line starts across (2748-2752, 2699-2702): the content
/// box's left, or in RTL its right less the line.
fn column_x(column: &Column) -> f32 {
    if column.direction.is_rtl() {
        column_width(column) - column.content_box_inset.right - line_cross(column)
    } else {
        column.content_box_inset.left
    }
}
