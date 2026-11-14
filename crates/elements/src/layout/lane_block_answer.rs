//! The block lane container's own answer from its run's fold, mirrored
//! from taffy 0.15.1 `compute/block.rs` (`compute_inner` 666-680,
//! 738-783, 832-850; `perform_final_layout_on_in_flow_children`
//! 1734-1752, 1808-1822) and `compute/common/scrollable_overflow.rs`
//! (64-70); `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeMath;
use taffy::{
    Baselines, CollapsibleMarginSet, LayoutOutput, OofCandidates, OofPositioningArea, Point, Rect,
    RunMode, Size,
};

use super::lane_block::Container;
use super::lane_block_summary::BlockSummary;
use super::lane_tree::Summary;
use crate::lane_decline::LaneDecline;

/// The container's answer from `run`, its children's fold. Every
/// child in the run is in flow and none collapses through, so the
/// run's `inner` is Taffy's committed offset from the first child's
/// top to the last's bottom.
///
/// # Errors
/// A scroll container with a child wholly before its origin, which
/// Taffy leaves out of the overflow (`BeforeScrollOrigin`).
pub(crate) fn lane_answer(c: &Container, run: &BlockSummary) -> Result<LayoutOutput, LaneDecline> {
    let a = &c.answer;
    let first_offset = if c.own_margins_collapse.start {
        0.0
    } else {
        run.top().resolve()
    };
    let first_child_top = f64::from(c.resolved_inset.top) + first_offset;
    let outer_height = outer_height(c, content_height(c, run, first_offset));
    let size = Size {
        width: c.outer_width,
        height: outer_height,
    };
    let (top_margin, bottom_margin) = margin_sets(c, run, outer_height);
    let baseline = run.bounds().baseline.map(|b| narrow(first_child_top + b));
    let mut output = LayoutOutput {
        size,
        scrollable_overflow_rect: Rect::ZERO,
        baselines: if a.suppresses_baseline {
            Baselines::NONE
        } else {
            Baselines::from_first(baseline)
        },
        top_margin,
        bottom_margin,
        // Taffy: every in-flow child collapses through. A child that
        // does is declined (`BlockSummary::leaf`) and an out-of-flow one
        // too (`lane_block_item.rs`), so only an empty run can; PR 3b
        // lifts the first decline and changes this line with it.
        margins_can_collapse_through: !a.prevents_collapse_through
            && outer_height == 0.0
            && run.is_empty(),
        oof_candidates: OofCandidates::NONE,
        oof_positioning_area: None,
    };
    if c.run_mode == RunMode::ComputeSize {
        return Ok(output);
    }
    output.oof_positioning_area = Some(positioning_area(c, size));
    output.scrollable_overflow_rect = overflow_rect(c, run, first_child_top)?;
    Ok(output)
}

/// The known height, else the content clamped, floored at the padding
/// and border (`compute_inner` 666-670).
fn outer_height(c: &Container, content_height: f32) -> f32 {
    let a = &c.answer;
    a.known_height
        .unwrap_or(content_height.maybe_clamp(a.min_size.height, a.max_size.height))
        .maybe_max(Some(a.padding_border_size.height))
}

/// The area inside the border and gutter that out-of-flow children are
/// placed in (`compute_inner` 832-837).
fn positioning_area(c: &Container, size: Size<f32>) -> OofPositioningArea {
    let inset = c.resolved_border + c.answer.scrollbar_gutter;
    OofPositioningArea {
        size: size - inset.sum_axes(),
        offset: Point {
            x: inset.left,
            y: inset.top,
        },
    }
}

/// The inset, the first child's offset unless it escapes, the run, the
/// last child's bottom margin unless it escapes, the inset
/// (`perform_final_layout_on_in_flow_children` 1810-1821).
fn content_height(c: &Container, run: &BlockSummary, first_offset: f64) -> f32 {
    let last_offset = if c.own_margins_collapse.end {
        0.0
    } else {
        run.bottom().resolve()
    };
    let height = f64::from(c.resolved_inset.top)
        + first_offset
        + run.inner()
        + last_offset
        + f64::from(c.resolved_inset.bottom);
    narrow(height.max(0.0))
}

/// The container's collapsible margin sets: its children's where its
/// own margins collapse with theirs, the bottom only when `min-height`
/// did not set the used height (`compute_inner` 672-679, 759-771).
fn margin_sets(
    c: &Container,
    run: &BlockSummary,
    outer_height: f32,
) -> (CollapsibleMarginSet, CollapsibleMarginSet) {
    let a = &c.answer;
    let constrained_by_min_height =
        matches!(a.min_size.height, Some(h) if h > 0.0 && h >= outer_height);
    let top = if c.own_margins_collapse.start {
        run.top().as_set()
    } else {
        CollapsibleMarginSet::from_margin(a.margins[0])
    };
    let bottom = if c.own_margins_collapse.end && !constrained_by_min_height {
        run.bottom().as_set()
    } else {
        CollapsibleMarginSet::from_margin(a.margins[1])
    };
    (top, bottom)
}

/// The in-flow children's scrollable overflow in the container, from
/// the run's extents: each child's contribution was taken from its
/// border box's start, which is the first child's top plus its offset
/// in the run, less the border, unioned with the origin
/// (`Rect::ZERO`), and a scroll container's end padding added. A
/// scroll container leaves out a child wholly before its origin
/// (`scrollable_overflow.rs` 64-70); the fold cannot, so such a
/// container is declined.
fn overflow_rect(
    c: &Container,
    run: &BlockSummary,
    first_child_top: f64,
) -> Result<Rect<f32>, LaneDecline> {
    let bounds = run.bounds();
    let origin = first_child_top - f64::from(c.resolved_border.top);
    if c.scrolls && (origin + bounds.end <= 0.0 || bounds.across_end <= 0.0) {
        return Err(LaneDecline::BeforeScrollOrigin);
    }
    let mut rect = Rect {
        left: narrow(bounds.across[0].min(0.0)),
        right: narrow(bounds.across[1].max(0.0)),
        top: narrow((origin + bounds.low).min(0.0)),
        bottom: narrow((origin + bounds.reach).max(0.0)),
    };
    if c.scrolls {
        let padding = c.answer.resolved_padding;
        rect.right += if c.direction.is_rtl() {
            padding.left
        } else {
            padding.right
        };
        rect.bottom += padding.bottom;
    }
    Ok(rect)
}

/// A sum in f64 narrowed to Taffy's f32.
#[allow(clippy::cast_possible_truncation)] // CSS px sums, well within f32
pub(crate) fn narrow(value: f64) -> f32 {
    value as f32
}
