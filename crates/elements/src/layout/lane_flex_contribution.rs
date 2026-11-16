//! What a lane column item adds to the column's height when the column
//! sizes itself, mirrored from taffy 0.15.1 `compute/flexbox.rs`
//! (`determine_container_main_size` 1420-1745) for one item:
//! `item_main_length` under definite space (1433-1439), and the content
//! contribution under min- or max-content space (1520-1716) for an item
//! that does not grow; `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeMath;
use taffy::{
    AbsoluteAxis, AlignItems, AvailableSpace, LayoutInput, Line, NodeId, RequestedAxis, RunMode,
    Size, SizingMode,
};

use super::lane_flex::Column;
use super::lane_flex_base::{Base, transferred};
use super::lane_flex_item::FlexChild;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

impl BoxTree<'_> {
    /// The item's two shares of the column's height: under definite
    /// space along, and under min- or max-content space. Both are kept
    /// in the leaf whatever the column was asked, as the leaf serves a
    /// later ask of either kind (a flex parent's size queries, then its
    /// layout); the answer picks one (`lane_flex_answer.rs`).
    ///
    /// # Errors
    /// The content share differs under min- and max-content space along
    /// (`SpaceDependentHeight`).
    pub(super) fn flex_contributions(
        &mut self,
        id: NodeId,
        column: &Column,
        child: &FlexChild,
        base: &Base,
    ) -> Result<[f32; 2], LaneDecline> {
        let most = self.content_share(id, (column, AvailableSpace::MaxContent), child, base);
        let least = self.content_share(id, (column, AvailableSpace::MinContent), child, base);
        // Exactly: the leaf is the same under both kinds, or it is not
        if most.to_bits() != least.to_bits() {
            return Err(LaneDecline::SpaceDependentHeight);
        }
        Ok([basis_length(child, base), most])
    }

    /// The flex basis plus the share the content adds above it (1660-
    /// 1716, with `flex-grow: 0`): the content contribution, or the
    /// basis and margins where shrinking is scaled by a zero basis.
    fn content_share(
        &mut self,
        id: NodeId,
        along: (&Column, AvailableSpace),
        child: &FlexChild,
        base: &Base,
    ) -> f32 {
        let margin_sum = child.margin.vertical_axis_sum();
        let contribution = self.content_contribution(id, along, child, base);
        let diff = contribution - margin_sum - base.flex_basis;
        let flex_contribution = if diff > 0.0 {
            diff
        } else if diff < 0.0 {
            let scaled_shrink_factor = child.flex_shrink.max(1.0) * base.inner_flex_basis;
            if scaled_shrink_factor == 0.0 {
                0.0
            } else {
                scaled_shrink_factor * (diff / scaled_shrink_factor)
            }
        } else {
            0.0
        };
        base.flex_basis + flex_contribution + margin_sum
    }

    /// The item's content contribution (1520-1658): its preferred height
    /// within its limits, else its basis when it scrolls, else its
    /// content measured, never below its basis.
    fn content_contribution(
        &mut self,
        id: NodeId,
        along: (&Column, AvailableSpace),
        child: &FlexChild,
        base: &Base,
    ) -> f32 {
        let margin_sum = child.margin.vertical_axis_sum();
        let (style_min, style_max) = (child.min_size.height, child.max_size.height);
        let (min, max) = limits(child, base);
        let preferred = child.size.height;
        match preferred {
            Some(pref) if max <= min || max <= pref => pref.min(max).max(min) + margin_sum,
            _ if max <= min => min + margin_sum,
            _ if child.is_scroll_container() => base.flex_basis + margin_sum,
            Some(pref) => {
                let padding_border =
                    child.padding.vertical_axis_sum() + child.border.vertical_axis_sum();
                (pref.max(padding_border).max(base.flex_basis) + margin_sum)
                    .maybe_clamp(style_min, style_max)
            }
            None => {
                let inner = self.measured_height(id, along, child);
                (inner.max(base.flex_basis) + margin_sum).maybe_clamp(style_min, style_max)
            }
        }
    }

    /// The item measured along the column for its contribution (1575-
    /// 1655) with `space` along, its height from its width by the
    /// aspect ratio at least.
    fn measured_height(
        &mut self,
        id: NodeId,
        (column, space): (&Column, AvailableSpace),
        child: &FlexChild,
    ) -> f32 {
        let cross_space = column
            .available_space
            .width
            .map_definite_value(|val| {
                (column.node_inner_size.width.unwrap_or(val) - child.margin.horizontal_axis_sum())
                    .max(0.0)
            })
            .maybe_clamp(child.min_size.width, child.max_size.width);
        let known = known_width(child, cross_space);
        let inputs = LayoutInput {
            known_dimensions: known,
            known_dimensions_are_definite: Size {
                width: true,
                height: true,
            },
            parent_size: column.node_inner_size,
            available_space: Size {
                width: cross_space,
                height: space,
            },
            sizing_mode: SizingMode::ContentSize,
            axis: RequestedAxis::from(AbsoluteAxis::Vertical),
            run_mode: RunMode::ComputeSize,
            vertical_margins_are_collapsible: Line::FALSE,
        };
        let measured = self.lay_out(id, inputs, None).size.height;
        let from_ratio = child
            .aspect_ratio
            .zip(known.width)
            .map(|(ratio, width)| width / ratio);
        measured.maybe_max(from_ratio)
    }
}

/// The item's `item_main_length` (1433-1439): its basis at least its
/// styled minimum, plus margins, at least its padding and border.
fn basis_length(child: &FlexChild, base: &Base) -> f32 {
    let padding_border = child.padding.vertical_axis_sum() + child.border.vertical_axis_sum();
    (base.flex_basis.maybe_max(child.min_size.height) + child.margin.vertical_axis_sum())
        .max(padding_border)
}

/// The item's minimum and maximum height for its contribution (1520-
/// 1550): the basis bounds it below when it does not shrink and above,
/// since it does not grow.
fn limits(child: &FlexChild, base: &Base) -> (f32, f32) {
    let resolved = base.resolved_minimum_main_size;
    let basis = Some(base.flex_basis);
    let basis_min = basis.filter(|_| child.flex_shrink == 0.0);
    let min = child
        .min_size
        .height
        .maybe_max(basis_min)
        .or(basis_min)
        .unwrap_or(resolved)
        .max(resolved);
    let max = child
        .max_size
        .height
        .maybe_min(basis)
        .or(basis)
        .unwrap_or(f32::INFINITY);
    (min, max)
}

/// The item's width when measured for its contribution (1596-1614):
/// its own, clamped by its transferred limits, else stretched to the
/// space across.
fn known_width(child: &FlexChild, cross_space: AvailableSpace) -> Size<Option<f32>> {
    let (min, max) = transferred(child);
    let mut known = Size {
        height: None,
        ..child.size
    };
    known.width = known.width.maybe_clamp(min.width, max.width);
    if child.align_self == AlignItems::STRETCH && known.width.is_none() {
        known.width = cross_space.into_option();
    }
    known
}
