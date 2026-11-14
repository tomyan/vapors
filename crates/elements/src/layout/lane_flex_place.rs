//! Where a lane column's item sits across the column and what it adds
//! to the column's overflow, mirrored from taffy 0.15.1
//! `compute/flexbox.rs` (`calculate_cross_size` 2131-2141,
//! `resolve_cross_axis_auto_margins` 2355-2408,
//! `align_flex_items_along_cross_axis` 2410-2502,
//! `determine_container_cross_size` 2504-2533, `calculate_flex_item`
//! 2662-2677) and `compute/common/scrollable_overflow.rs` (25-63);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeMath;
use taffy::{AlignItemsKeyword, LayoutOutput, Point, Rect};

use super::lane_block_child_place::{end_extent, propagates, start_extent};
use super::lane_bounds::Bounds;
use super::lane_flex::Column;
use super::lane_flex_item::FlexChild;

/// The line's width: the column's, clamped, less its inset (2131-2141).
pub(super) fn line_cross(column: &Column) -> f32 {
    column
        .node_outer_size
        .width
        .maybe_clamp(column.min_size.width, column.max_size.width)
        .maybe_sub(column.content_box_inset.horizontal_axis_sum())
        .maybe_max(0.0)
        .unwrap_or(0.0)
}

/// The column's outer width (2516-2523).
pub(super) fn column_width(column: &Column) -> f32 {
    let inset = column.content_box_inset.horizontal_axis_sum();
    column
        .node_outer_size
        .width
        .unwrap_or(0.0)
        .maybe_clamp(column.min_size.width, column.max_size.width)
        .max(inset - column.scrollbar_gutter.x)
}

/// The item's margins once its `auto` margins across took the free
/// space, and its offset across when it has none (2367-2405).
pub(super) fn cross_place(column: &Column, child: &FlexChild, width: f32) -> (Rect<f32>, f32) {
    let free = line_cross(column) - (width + child.margin.horizontal_axis_sum());
    let mut margin = child.margin;
    match (child.margin_is_auto.left, child.margin_is_auto.right) {
        (true, true) => {
            margin.left = free / 2.0;
            margin.right = free / 2.0;
        }
        (true, false) => margin.left = free,
        (false, true) => margin.right = free,
        (false, false) => return (margin, aligned(column, child, free)),
    }
    (margin, 0.0)
}

/// The item's offset across by its `align-self` (2410-2502), for a
/// single-line column: `start` sides swap in RTL.
fn aligned(column: &Column, child: &FlexChild, free: f32) -> f32 {
    let rtl = column.direction.is_rtl();
    let keyword = if child.align_self.is_safe() && free < 0.0 {
        AlignItemsKeyword::Start
    } else {
        child.align_self.keyword
    };
    match keyword {
        AlignItemsKeyword::Start
        | AlignItemsKeyword::FlexStart
        | AlignItemsKeyword::Baseline
        | AlignItemsKeyword::Stretch => {
            if rtl {
                free
            } else {
                0.0
            }
        }
        AlignItemsKeyword::End | AlignItemsKeyword::FlexEnd => {
            if rtl {
                0.0
            } else {
                free
            }
        }
        AlignItemsKeyword::Center => free / 2.0,
        AlignItemsKeyword::Normal | AlignItemsKeyword::SelfStart | AlignItemsKeyword::SelfEnd => {
            unreachable!("`align-self` is resolved to a flex keyword (`lane_flex_item.rs`)")
        }
    }
}

/// What the item adds to the column's overflow and baseline, from its
/// border box's top along the column and from the column's border
/// across (2662-2677, `scrollable_overflow.rs` 25-63). An empty extent
/// adds nothing, and nothing to the earliest end either.
pub(super) fn item_bounds(
    column: &Column,
    child: &FlexChild,
    output: &LayoutOutput,
    location: Point<f32>,
    baseline: f32,
) -> Bounds {
    let reaches = propagates(child.overflow, child.contain);
    let end = end_extent(output.size, output.scrollable_overflow_rect, reaches);
    let baseline = Some(f64::from(baseline));
    if end.width <= 0.0 || end.height <= 0.0 {
        return Bounds {
            baseline,
            ..Bounds::EMPTY
        };
    }
    let start = start_extent(output.scrollable_overflow_rect, reaches);
    let x = if column.direction.is_rtl() {
        column_width(column) - (location.x + output.size.width) - column.border.right
    } else {
        location.x - column.border.left
    };
    let along = [location.y + start.y, location.y + end.height].map(f64::from);
    let across = [x + start.x, x + end.width].map(f64::from);
    Bounds {
        low: along[0],
        reach: along[1],
        end: along[1],
        across,
        across_end: across[1],
        baseline,
        cross: [0.0; 2],
    }
}
