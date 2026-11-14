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
//! Taffy is MIT licensed, and the mirrored lines in the
//! `lane_block*.rs` and `lane_flex*.rs` modules carry its notice:
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

use taffy::util::ResolveOrZero;
use taffy::{
    AlignItems, CoreStyle, Direction, LayoutInput, Line, NodeId, Rect, RunMode, Size, TextAlign,
};

use super::lane_block_container::{Edges, answering, container};
use super::lane_block_sizes::{known_dimensions, short_answer, sizes};
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
    /// Whether the container scrolls: its end padding joins its
    /// overflow, and a child wholly before its origin declines it.
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
        let sizes = sizes(
            style,
            inputs.parent_size,
            (padding + border).sum_axes(),
            calc,
        );
        let known = known_dimensions(&inputs, &sizes, (padding + border).sum_axes());
        let outer_width = match (short_answer(&inputs, known), known.width) {
            (Some(short), _) => return Settled::Short(short),
            (None, None) => return Settled::Declined(LaneDecline::WidthUnknown),
            (None, Some(width)) => width,
        };
        let resolution_width = parent_width.unwrap_or(outer_width);
        let edges = Edges::of(style, (padding, border), resolution_width, calc);
        let answer = answering(style, &inputs, &sizes, &edges, in_block, known.height, calc);
        Settled::Laid(container(
            style,
            &inputs,
            &sizes,
            &edges,
            (outer_width, known),
            answer,
        ))
    }
}

#[cfg(test)]
#[path = "lane_block_tests.rs"]
mod tests;
