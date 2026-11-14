//! A lane column item's leaf for the lane's tree and its layout,
//! mirrored from taffy 0.15.1 `compute/flexbox.rs` (`calculate_flex_item`
//! 2611-2677) and `compute/common/alignment.rs` (12-18);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::{Layout, LayoutOutput, Overflow, Point, Rect, Size};

use super::lane_flex::Column;
use super::lane_flex_base::Base;
use super::lane_flex_item::FlexChild;
use super::lane_flex_place::item_bounds;
use super::lane_flex_summary::FlexItem;

/// The item's leaf: its margins, height, shares of the column's height,
/// whether it could shrink, and what it adds to the column's overflow
/// and baseline from its border box's top.
pub(super) fn leaf(
    column: &Column,
    (child, base): (&FlexChild, &Base),
    output: &LayoutOutput,
    placed: (Point<f32>, Rect<f32>),
    [basis, content]: [f32; 2],
) -> FlexItem {
    let height = output.size.height;
    let baseline = first_baseline(output.baselines.first, height, child.overflow.y);
    FlexItem {
        margins: [f64::from(placed.1.top), f64::from(placed.1.bottom)],
        size: f64::from(height),
        basis: f64::from(basis),
        content: f64::from(content),
        shrinkable: child.flex_shrink > 0.0
            && base.flex_basis >= base.hypothetical_inner
            && base.resolved_minimum_main_size < base.hypothetical_inner,
        bounds: item_bounds(column, child, output, placed.0, baseline),
    }
}

/// The item's first baseline from its border box's top, its height
/// when it has none (2611-2621, `child_first_baseline`).
fn first_baseline(first: Option<f32>, height: f32, overflow_y: Overflow) -> f32 {
    let first = if overflow_y.is_scroll_container() {
        Some(first.unwrap_or(height).min(height).max(0.0))
    } else {
        first
    };
    first.unwrap_or(height)
}

/// The item's layout (2632-2647). Taffy numbers items by their place
/// in the list; nothing in vapors reads it, so every lane item is 0,
/// as in the block lane.
pub(super) fn item_layout(
    child: &FlexChild,
    output: &LayoutOutput,
    location: Point<f32>,
    margin: Rect<f32>,
) -> Layout {
    let scrollbar = |axis: Overflow| (axis == Overflow::Scroll).then_some(child.scrollbar_width);
    Layout {
        order: 0,
        size: output.size,
        scrollable_overflow_rect: output.scrollable_overflow_rect,
        scrollbar_size: Size {
            width: scrollbar(child.overflow.y).unwrap_or(0.0),
            height: scrollbar(child.overflow.x).unwrap_or(0.0),
        },
        location,
        padding: child.padding,
        border: child.border,
        margin,
    }
}
