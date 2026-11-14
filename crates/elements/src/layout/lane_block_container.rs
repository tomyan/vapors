//! A block lane container's shared inputs and the inputs of its own
//! answer, mirrored from taffy 0.15.1 `compute/block.rs`
//! (`compute_inner` 493-633, 759-770); `lane_block.rs` carries Taffy's
//! MIT notice.

use taffy::util::{MaybeMath, ResolveOrZero};
use taffy::{BlockContainerStyle, CoreStyle, LayoutInput, Rect, Size, SizingMode};

use super::lane_block::{Answering, Container};
use super::lane_block_sizes::{
    Sizes, collapsing, establishes_new_bfc, is_scroll_container, scrollbar_gutter,
};

/// The container's edges resolved two ways: against the parent's
/// width, and against the percentage-resolution width (`compute_inner`
/// 493-512, 628-633).
pub(super) struct Edges {
    padding: Rect<f32>,
    border: Rect<f32>,
    gutter: Rect<f32>,
    resolved_padding: Rect<f32>,
    resolved_border: Rect<f32>,
}

impl Edges {
    /// The edges as resolved against the parent's width, and resolved
    /// again against `resolution_width`.
    pub(super) fn of(
        style: &taffy::Style,
        (padding, border): (Rect<f32>, Rect<f32>),
        resolution_width: f32,
        calc: impl Fn(*const (), f32) -> f32 + Copy,
    ) -> Self {
        Self {
            padding,
            border,
            gutter: scrollbar_gutter(style),
            resolved_padding: style
                .padding()
                .resolve_or_zero(Some(resolution_width), calc),
            resolved_border: style.border().resolve_or_zero(Some(resolution_width), calc),
        }
    }

    fn inset(&self) -> Rect<f32> {
        self.padding + self.border + self.gutter
    }

    fn resolved_inset(&self) -> Rect<f32> {
        self.resolved_padding + self.resolved_border + self.gutter
    }
}

/// What the answer reads besides the fold (`compute_inner` 580-586,
/// 759-770).
pub(super) fn answering(
    style: &taffy::Style,
    inputs: &LayoutInput,
    sizes: &Sizes,
    edges: &Edges,
    in_block: bool,
    known_height: Option<f32>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Answering {
    let (padding, border) = (edges.padding, edges.border);
    let prevents_collapse_through = !style.is_block()
        || !in_block
        || establishes_new_bfc(style)
        || style.position().is_out_of_flow()
        || padding.top > 0.0
        || padding.bottom > 0.0
        || border.top > 0.0
        || border.bottom > 0.0;
    let margin = style.margin();
    Answering {
        known_height,
        min_size: sizes.min,
        max_size: sizes.max,
        padding_border_size: (padding + border).sum_axes(),
        resolved_padding: edges.resolved_padding,
        scrollbar_gutter: edges.gutter,
        margins: [
            margin.top.resolve_or_zero(inputs.parent_size.width, calc),
            margin
                .bottom
                .resolve_or_zero(inputs.parent_size.width, calc),
        ],
        prevents_collapse_through,
        suppresses_baseline: style.contain().suppresses_baseline(),
    }
}

/// What the children share (`compute_inner` 547-551, 563-578,
/// 618-620; `perform_final_layout_on_in_flow_children` 1103-1106).
pub(super) fn container(
    style: &taffy::Style,
    inputs: &LayoutInput,
    sizes: &Sizes,
    edges: &Edges,
    (outer_width, known): (f32, Size<Option<f32>>),
    answer: Answering,
) -> Container {
    let percentage_basis = Size {
        width: known.width,
        height: known
            .height
            .filter(|_| inputs.known_dimensions_are_definite.height),
    };
    let inherent = inputs.sizing_mode == SizingMode::InherentSize;
    let percentage_resolution_height = percentage_basis.height.or(inherent
        .then(|| sizes.size.height.maybe_max(sizes.min.height))
        .flatten());
    let resolved_inset = edges.resolved_inset();
    let inner_width = (outer_width - resolved_inset.horizontal_axis_sum()).max(0.0);
    let (children_adjoin, own_margins_collapse) = collapsing(
        style,
        inputs,
        (edges.padding, edges.border),
        sizes.size.height.is_none(),
    );
    Container {
        run_mode: inputs.run_mode,
        outer_width,
        inner_width,
        parent_size: Size {
            width: Some(inner_width),
            height: percentage_resolution_height.maybe_sub(resolved_inset.vertical_axis_sum()),
        },
        item_basis: percentage_basis.maybe_sub(edges.inset().sum_axes()),
        content_box_inset: edges.inset(),
        resolved_inset,
        resolved_border: edges.resolved_border,
        direction: style.direction(),
        text_align: style.text_align(),
        justify_items: style.justify_items(),
        own_margins_collapse,
        children_adjoin,
        scrolls: is_scroll_container(style),
        answer,
    }
}
