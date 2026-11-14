//! A lane column's own answer from its items' fold, mirrored from taffy
//! 0.15.1 `compute/flexbox.rs` (`compute_preliminary` 414-445, 528-585,
//! `determine_container_main_size` 1420-1745, `resolve_flexible_lengths`
//! 1747-1770, `determine_container_cross_size` 2504-2533,
//! `final_layout_pass` 2789-2800) and `compute/common/
//! scrollable_overflow.rs` (64-70); `lane_block.rs` carries Taffy's MIT
//! notice.

use taffy::util::MaybeMath;
use taffy::{
    AvailableSpace, Baselines, LayoutOutput, OofPositioningArea, Point, Rect, RequestedAxis,
    RunMode, Size,
};

use super::lane_block_answer::narrow;
use super::lane_flex::Column;
use super::lane_flex_place::column_width;
use super::lane_flex_summary::FlexSummary;
use super::lane_tree::Summary;
use crate::lane_decline::LaneDecline;

/// The column's answer from `run`, its items' fold.
///
/// # Errors
/// The column has no items (`NoItems`); the items overflow the column's
/// height and one can shrink (`Shrinks`); or a scroll container has an item wholly before its
/// origin (`BeforeScrollOrigin`): Taffy lays the column out.
pub(crate) fn lane_flex_answer(
    column: &Column,
    run: &FlexSummary,
) -> Result<LayoutOutput, LaneDecline> {
    if run.is_empty() {
        return Err(LaneDecline::NoItems);
    }
    let gap = f64::from(column.gap);
    let (outer_height, inner_height) = main_size(column, run, gap);
    if column.run_mode == RunMode::ComputeSize && column.axis == RequestedAxis::Vertical {
        return Ok(LayoutOutput::from_outer_size(Size {
            width: 0.0,
            height: outer_height,
        }));
    }
    if run.outer(gap) > f64::from(inner_height) && run.shrinkable() {
        return Err(LaneDecline::Shrinks);
    }
    let size = Size {
        width: column_width(column),
        height: outer_height,
    };
    if column.run_mode == RunMode::ComputeSize {
        return Ok(LayoutOutput::from_outer_size(size));
    }
    let first_top = f64::from(column.content_box_inset.top) + run.leading();
    let overflow = overflow_rect(column, run, first_top)?;
    let baselines = baselines(column, run, first_top);
    let mut output = LayoutOutput::from_sizes_and_baselines(size, overflow, baselines);
    output.oof_positioning_area = Some(positioning_area(column, size));
    Ok(output)
}

/// The first item's baseline, from its border box's top at
/// `first_top` (560-576), unless containment suppresses it (342-345).
fn baselines(column: &Column, run: &FlexSummary, first_top: f64) -> Baselines {
    if column.suppresses_baseline {
        return Baselines::NONE;
    }
    Baselines::from_first(run.bounds().baseline.map(|b| narrow(first_top + b)))
}

/// The column's outer and inner height: given, else from its items'
/// shares, clamped (414-427, 1420-1745).
fn main_size(column: &Column, run: &FlexSummary, gap: f64) -> (f32, f32) {
    let inset = column.content_box_inset.vertical_axis_sum();
    if let Some(height) = column.node_outer_size.height {
        let inner = height - inset;
        return (inner + inset, inner);
    }
    let [basis, content] = run.contributions(gap);
    let items = match column.available_space.height {
        AvailableSpace::Definite(_) => narrow(basis),
        _ => narrow(content).max(0.0),
    };
    let outer = (items + inset)
        .maybe_clamp(column.min_size.height, column.max_size.height)
        .max(inset - column.scrollbar_gutter.y);
    (outer, (outer - inset).max(0.0))
}

/// The items' scrollable overflow in the column, from the run's
/// extents measured from the first item's border box (`first_top`),
/// unioned with the origin, and a scroll container's end padding
/// added (2789-2800). A scroll container leaves out an item wholly
/// before its origin; the fold cannot, so such a column is declined.
fn overflow_rect(
    column: &Column,
    run: &FlexSummary,
    first_top: f64,
) -> Result<Rect<f32>, LaneDecline> {
    let bounds = run.bounds();
    let origin = first_top - f64::from(column.border.top);
    if column.scrolls && (origin + bounds.end <= 0.0 || bounds.across_end <= 0.0) {
        return Err(LaneDecline::BeforeScrollOrigin);
    }
    let mut rect = Rect {
        left: narrow(bounds.across[0].min(0.0)),
        right: narrow(bounds.across[1].max(0.0)),
        top: narrow((origin + bounds.low).min(0.0)),
        bottom: narrow((origin + bounds.reach).max(0.0)),
    };
    if column.scrolls {
        let (inset, border, gutter) = (
            column.content_box_inset,
            column.border,
            column.scrollbar_gutter,
        );
        rect.right += if column.direction.is_rtl() {
            inset.left - border.left - gutter.x
        } else {
            inset.right - border.right - gutter.x
        };
        rect.bottom += inset.bottom - border.bottom - gutter.y;
    }
    Ok(rect)
}

/// The area inside the border and the scrollbar gutter that out-of-
/// flow children are placed in (528-537).
fn positioning_area(column: &Column, size: Size<f32>) -> OofPositioningArea {
    let gutter = column.scrollbar_gutter;
    let rtl = column.direction.is_rtl();
    let inset = column.border
        + Rect {
            left: if rtl { gutter.x } else { 0.0 },
            right: if rtl { 0.0 } else { gutter.x },
            top: 0.0,
            bottom: gutter.y,
        };
    OofPositioningArea {
        size: size - inset.sum_axes(),
        offset: Point {
            x: inset.left,
            y: inset.top,
        },
    }
}
