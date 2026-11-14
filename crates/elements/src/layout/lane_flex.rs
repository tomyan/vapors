//! A flex column laid out one item at a time (`docs/plans/splice-
//! layout.md`, section 10): the column's own inputs, mirrored from taffy
//! 0.15.1 `compute/flexbox.rs` (`compute_flexbox_layout` 255-348,
//! `compute_constants` 634-749, `determine_available_space` 896-945),
//! for a column whose items' sizes and positions do not depend on the
//! total (10.3); every other column is declined by name and laid out by
//! Taffy. The right-to-left branches (`column_x`, `aligned`,
//! `item_bounds`, `relative_inset`) are mirrored but unverified: vapors'
//! CSS has no `direction` property, so no page reaches them. Each item
//! is `lane_flex_item.rs`, `lane_flex_base.rs` and
//! `lane_flex_child.rs`; the column's answer `lane_flex_answer.rs`.
//! `lane_block.rs` carries Taffy's MIT notice, which covers the
//! mirrored lines of the `lane_flex*.rs` modules too.

use taffy::{
    AlignItems, AvailableSpace, Direction, LayoutInput, NodeId, Point, Rect, RequestedAxis,
    RunMode, Size,
};

use super::lane_flex_column::{column_declines, constants};
use super::lane_flex_sizes::{short_answer, styled_sizes};
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// What every item of a lane column shares, and what the column's
/// answer needs besides the fold (Taffy's `AlgoConstants` for a
/// single-line column of known width).
pub(crate) struct Column {
    pub(crate) run_mode: RunMode,
    pub(crate) axis: RequestedAxis,
    pub(crate) direction: Direction,
    pub(crate) min_size: Size<Option<f32>>,
    pub(crate) max_size: Size<Option<f32>>,
    pub(crate) border: Rect<f32>,
    pub(crate) content_box_inset: Rect<f32>,
    pub(crate) scrollbar_gutter: Point<f32>,
    pub(crate) scrolls: bool,
    /// The column's `row-gap`, between neighbouring items.
    pub(crate) gap: f32,
    pub(crate) align_items: AlignItems,
    /// The column's outer and inner size as known when it was asked:
    /// the width always, the height when definite or given.
    pub(crate) node_outer_size: Size<Option<f32>>,
    pub(crate) node_inner_size: Size<Option<f32>>,
    /// Which of its given sizes are definite, a size it was not given
    /// counting as definite (`compute_flexbox_layout` 331-333).
    pub(crate) definite: Size<bool>,
    /// Whether its parent gave it a definite width to lay out in.
    pub(crate) outer_space_width_is_definite: bool,
    /// The space the items are laid out in (`determine_available_space`).
    pub(crate) available_space: Size<AvailableSpace>,
    pub(crate) suppresses_baseline: bool,
}

/// How a column was settled for one way it was asked.
pub(crate) enum FlexSettled {
    /// Its items are laid out with these inputs.
    Laid(Column),
    /// Its size was known without its items (a `ComputeSize` ask).
    Short(taffy::LayoutOutput),
    /// Not a lane this time; Taffy lays it out.
    Declined(LaneDecline),
}

impl Column {
    /// Its height, when given, resolves its items' percentages
    /// (`known_main_size_is_definite`, 698).
    pub(crate) fn known_main_size_is_definite(&self) -> bool {
        self.definite.height
    }

    pub(crate) fn has_definite_main_size(&self) -> bool {
        self.definite.height && self.node_outer_size.height.is_some()
    }

    pub(crate) fn has_definite_cross_size(&self) -> bool {
        self.definite.width && self.node_outer_size.width.is_some()
    }

    pub(crate) fn cross_axis_available_space_is_definite(&self) -> bool {
        self.has_definite_cross_size() || self.outer_space_width_is_definite
    }
}

impl BoxTree<'_> {
    /// Settles the flex container at `id` asked with `inputs`
    /// (`compute_flexbox_layout` 255-348).
    pub(crate) fn lane_column(&self, id: NodeId, inputs: LayoutInput) -> FlexSettled {
        let style = &self.boxes[usize::from(id)].style;
        let calc = |val, basis| self.calcs.resolve(val, basis);
        if let Err(decline) = column_declines(style) {
            return FlexSettled::Declined(decline);
        }
        let known = styled_sizes(style, &inputs, calc);
        if let Some(short) = short_answer(&inputs, known) {
            return FlexSettled::Short(short);
        }
        if known.width.is_none() {
            return FlexSettled::Declined(LaneDecline::WidthUnknown);
        }
        match constants(style, &inputs, known, calc) {
            Ok(column) => FlexSettled::Laid(column),
            Err(decline) => FlexSettled::Declined(decline),
        }
    }
}

#[cfg(test)]
#[path = "lane_flex_tests.rs"]
mod tests;
