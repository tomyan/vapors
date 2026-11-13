//! The block lane container's own answer from its run's fold, mirrored
//! from taffy 0.15.1 `compute/block.rs` (`compute_inner` 666-680,
//! 738-783, 832-850; `perform_final_layout_on_in_flow_children`
//! 1734-1752, 1808-1822); `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeMath;
use taffy::{
    Baselines, CollapsibleMarginSet, LayoutOutput, OofCandidates, OofPositioningArea, Point, Rect,
    RunMode, Size,
};

use super::lane_block::Container;
use super::lane_block_summary::BlockSummary;
use super::lane_tree::Summary;

/// The container's answer from `run`, its children's fold. Every
/// child in the run is in flow and none collapses through, so the
/// run's `inner` is Taffy's committed offset from the first child's
/// top to the last's bottom.
pub(crate) fn lane_answer(c: &Container, run: &BlockSummary) -> LayoutOutput {
    let a = &c.answer;
    let first_offset = if c.own_margins_collapse.start {
        0.0
    } else {
        run.top().resolve()
    };
    let last_offset = if c.own_margins_collapse.end {
        0.0
    } else {
        run.bottom().resolve()
    };
    let content_height = f64::from(c.resolved_inset.top)
        + first_offset
        + run.inner()
        + last_offset
        + f64::from(c.resolved_inset.bottom);
    let content_height = narrow(content_height.max(0.0));
    let outer_height = a
        .known_height
        .unwrap_or(content_height.maybe_clamp(a.min_size.height, a.max_size.height))
        .maybe_max(Some(a.padding_border_size.height));
    let size = Size {
        width: c.outer_width,
        height: outer_height,
    };
    let constrained_by_min_height =
        matches!(a.min_size.height, Some(h) if h > 0.0 && h >= outer_height);
    let bottom_collapses = c.own_margins_collapse.end && !constrained_by_min_height;
    let first_child_top = f64::from(c.resolved_inset.top) + first_offset;
    let baseline = run.bounds().baseline.map(|b| narrow(first_child_top + b));
    let mut output = LayoutOutput {
        size,
        scrollable_overflow_rect: Rect::ZERO,
        baselines: if a.suppresses_baseline {
            Baselines::NONE
        } else {
            Baselines::from_first(baseline)
        },
        top_margin: if c.own_margins_collapse.start {
            run.top().as_set()
        } else {
            CollapsibleMarginSet::from_margin(a.margins[0])
        },
        bottom_margin: if bottom_collapses {
            run.bottom().as_set()
        } else {
            CollapsibleMarginSet::from_margin(a.margins[1])
        },
        margins_can_collapse_through: !a.prevents_collapse_through
            && outer_height == 0.0
            && run.is_empty(),
        oof_candidates: OofCandidates::NONE,
        oof_positioning_area: None,
    };
    if c.run_mode == RunMode::ComputeSize {
        return output;
    }
    let inset = c.resolved_border + a.scrollbar_gutter;
    output.oof_positioning_area = Some(OofPositioningArea {
        size: size - inset.sum_axes(),
        offset: Point {
            x: inset.left,
            y: inset.top,
        },
    });
    output.scrollable_overflow_rect = overflow_rect(c, run, first_child_top);
    output
}

/// The in-flow children's scrollable overflow in the container, from
/// the run's extents: each child's contribution was taken from its
/// border box's start, which is the first child's top plus its offset
/// in the run, less the border, unioned with the origin
/// (`Rect::ZERO`), and a scroll container's end padding added.
fn overflow_rect(c: &Container, run: &BlockSummary, first_child_top: f64) -> Rect<f32> {
    let bounds = run.bounds();
    let origin = first_child_top - f64::from(c.resolved_border.top);
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
    rect
}

/// A sum in f64 narrowed to Taffy's f32.
#[allow(clippy::cast_possible_truncation)] // CSS px sums, well within f32
pub(crate) fn narrow(value: f64) -> f32 {
    value as f32
}
