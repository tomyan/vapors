//! The block lane's container: what every child shares, settled once
//! from the container's style and the way it was asked, and the
//! container's own answer from its run's fold (`docs/plans/
//! splice-layout.md`, sections 2 (a), (d) and 10.4). Mirrored from
//! taffy 0.15.1 `compute/block.rs`: `compute_block_layout` (380-474),
//! `compute_inner` (484-633, 666-680, 738-783, 832-850) and the end of
//! `perform_final_layout_on_in_flow_children` (1103-1115, 1808-1822),
//! without floats (off), `align-content` (declined at build) and
//! out-of-flow children (declined, `lane_block_child.rs`). The
//! `=0.15.1` pin in `Cargo.toml` makes a bump a re-read of these lines.
//!
//! Taffy is MIT licensed, and the mirrored lines here and in
//! `lane_block_child.rs` carry its notice:
//!
//! Copyright (c) 2018 Visly Inc. Copyright (c) 2026 Taffy Authors.
//! Permission is hereby granted, free of charge, to any person
//! obtaining a copy of this software and associated documentation
//! files (the "Software"), to deal in the Software without
//! restriction, including without limitation the rights to use, copy,
//! modify, merge, publish, distribute, sublicense, and/or sell copies
//! of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions: The above
//! copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software. THE SOFTWARE
//! IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
//! NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
//! BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
//! ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
//! CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.

use taffy::util::{MaybeMath, ResolveOrZero};
use taffy::{
    AlignItems, BlockContainerStyle, CoreStyle, Direction, LayoutInput, Line, NodeId, Rect,
    RunMode, Size, SizingMode, TextAlign,
};

use super::lane_block_sizes::{
    collapsing, establishes_new_bfc, is_scroll_container, known_dimensions, scrollbar_gutter,
    short_answer, sizes,
};
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// What every child of a lane container shares, and what its answer
/// needs besides the fold.
pub(crate) struct Container {
    pub(crate) run_mode: RunMode,
    pub(crate) outer_width: f32,
    pub(crate) inner_width: f32,
    /// The children's `parent_size`: the inner width and the
    /// percentage-resolution height less the inset.
    pub(crate) parent_size: Size<Option<f32>>,
    /// The children's percentage basis for their own sizes
    /// (`generate_item_list`'s `node_inner_size`).
    pub(crate) item_basis: Size<Option<f32>>,
    /// Padding, border and scrollbar gutter resolved against the
    /// parent's width, and against the percentage-resolution width.
    pub(crate) content_box_inset: Rect<f32>,
    pub(crate) resolved_inset: Rect<f32>,
    pub(crate) resolved_border: Rect<f32>,
    pub(crate) direction: Direction,
    pub(crate) text_align: TextAlign,
    pub(crate) justify_items: AlignItems,
    /// Whether the container's own margins collapse with its first
    /// and last child's, and whether children's margins adjoin its
    /// edges.
    pub(crate) own_margins_collapse: Line<bool>,
    pub(crate) children_adjoin: Line<bool>,
    /// Whether the container scrolls: its end padding joins its overflow.
    pub(crate) scrolls: bool,
    pub(super) answer: Answering,
}

/// What the container's own answer reads besides the fold.
pub(super) struct Answering {
    pub(super) known_height: Option<f32>,
    pub(super) min_size: Size<Option<f32>>,
    pub(super) max_size: Size<Option<f32>>,
    pub(super) padding_border_size: Size<f32>,
    pub(super) resolved_padding: Rect<f32>,
    pub(super) scrollbar_gutter: Rect<f32>,
    /// Its own top and bottom margins, for where they do not collapse
    /// with its children's.
    pub(super) margins: [f32; 2],
    pub(super) prevents_collapse_through: bool,
    pub(super) suppresses_baseline: bool,
}

/// How a container was settled for one way it was asked.
pub(crate) enum Settled {
    /// Its children are laid out with these inputs.
    Laid(Container),
    /// Its size was known without its children (a `ComputeSize` ask).
    Short(taffy::LayoutOutput),
    /// Not a lane this time; Taffy lays it out.
    Declined(LaneDecline),
}

impl BoxTree<'_> {
    /// Settles the container at `id` asked with `inputs`, `in_block`
    /// when inside its parent's block formatting context
    /// (`compute_block_layout` 380-446, `compute_inner` 484-633 and
    /// 1103-1106).
    pub(crate) fn lane_container(
        &self,
        id: NodeId,
        inputs: LayoutInput,
        in_block: bool,
    ) -> Settled {
        let style = &self.boxes[usize::from(id)].style;
        let calc = |val, basis| self.calcs.resolve(val, basis);
        let parent_width = inputs.parent_size.width;
        let padding = style.padding().resolve_or_zero(parent_width, calc);
        let border = style.border().resolve_or_zero(parent_width, calc);
        let padding_border_size = (padding + border).sum_axes();
        let sizes = sizes(style, inputs.parent_size, padding_border_size, calc);
        let known = known_dimensions(&inputs, &sizes, padding_border_size);
        if let Some(short) = short_answer(&inputs, known) {
            return Settled::Short(short);
        }
        let Some(outer_width) = known.width else {
            return Settled::Declined(LaneDecline::WidthUnknown);
        };
        let gutter = scrollbar_gutter(style);
        let percentage_basis = Size {
            width: known.width,
            height: known
                .height
                .filter(|_| inputs.known_dimensions_are_definite.height),
        };
        let percentage_resolution_width = parent_width.unwrap_or(outer_width);
        let resolved_padding = style
            .padding()
            .resolve_or_zero(Some(percentage_resolution_width), calc);
        let resolved_border = style
            .border()
            .resolve_or_zero(Some(percentage_resolution_width), calc);
        let resolved_inset = resolved_padding + resolved_border + gutter;
        let (children_adjoin, own_margins_collapse) = collapsing(
            style,
            &inputs,
            (padding, border),
            sizes.size.height.is_none(),
        );
        let prevents_collapse_through = !style.is_block()
            || !in_block
            || establishes_new_bfc(style)
            || style.position().is_out_of_flow()
            || padding.top > 0.0
            || padding.bottom > 0.0
            || border.top > 0.0
            || border.bottom > 0.0;
        let percentage_resolution_height = percentage_basis
            .height
            .or((inputs.sizing_mode == SizingMode::InherentSize)
                .then(|| sizes.size.height.maybe_max(sizes.min.height))
                .flatten());
        let inner_width = (outer_width - resolved_inset.horizontal_axis_sum()).max(0.0);
        let margin = style.margin();
        Settled::Laid(Container {
            run_mode: inputs.run_mode,
            outer_width,
            inner_width,
            parent_size: Size {
                width: Some(inner_width),
                height: percentage_resolution_height.maybe_sub(resolved_inset.vertical_axis_sum()),
            },
            item_basis: percentage_basis.maybe_sub((padding + border + gutter).sum_axes()),
            content_box_inset: padding + border + gutter,
            resolved_inset,
            resolved_border,
            direction: style.direction(),
            text_align: style.text_align(),
            justify_items: style.justify_items(),
            own_margins_collapse,
            children_adjoin,
            scrolls: is_scroll_container(style),
            answer: Answering {
                known_height: known.height,
                min_size: sizes.min,
                max_size: sizes.max,
                padding_border_size,
                resolved_padding,
                scrollbar_gutter: gutter,
                margins: [
                    margin.top.resolve_or_zero(parent_width, calc),
                    margin.bottom.resolve_or_zero(parent_width, calc),
                ],
                prevents_collapse_through,
                suppresses_baseline: style.contain().suppresses_baseline(),
            },
        })
    }
}

#[cfg(test)]
#[path = "lane_block_tests.rs"]
mod tests;
