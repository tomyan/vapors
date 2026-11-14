//! What a block lane's leaves depend on besides the children
//! themselves: the container's inputs, split into the height bases and
//! the rest, since a flex parent asks with the height unknown and then
//! lays out with it known, and only children whose styles read the
//! height bases depend on them.

use rustc_hash::FxHashMap;
use taffy::{
    AlignItems, Direction, Layout, LayoutOutput, Line, NodeId, Rect, RunMode, Style, TextAlign,
};

use super::lane::{Lane, Lanes};
use super::lane_block::Container;
use super::lane_block_answer::lane_answer;
use super::lane_block_summary::BlockSummary;
use super::lane_kind::LaneKind;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// The block lane (`lane_block*.rs`).
#[derive(Debug)]
pub(crate) struct BlockLane;

impl LaneKind for BlockLane {
    type Summary = BlockSummary;
    type Container = Container;
    type Share = Share;
    type Heights = Heights;

    fn share(c: &Container) -> Share {
        c.share()
    }

    fn heights(c: &Container) -> Heights {
        c.heights()
    }

    /// `row-gap` does not apply to block layout.
    fn gap(_: &Container) -> f64 {
        0.0
    }

    fn writes(c: &Container) -> bool {
        c.run_mode == RunMode::PerformLayout
    }

    /// The container's inset and, when its top margin collapses with its
    /// first child's, less that margin, which Taffy applies outside the
    /// container.
    fn top(c: &Container, run: &BlockSummary) -> f64 {
        let escape = if c.own_margins_collapse.start {
            run.top().resolve()
        } else {
            0.0
        };
        f64::from(c.resolved_inset.top) - escape
    }

    fn lay_out_child(
        tree: &mut BoxTree<'_>,
        slot: usize,
        c: &Container,
    ) -> Result<(BlockSummary, Layout), LaneDecline> {
        let laid = tree.lane_child(NodeId::from(slot), c)?;
        Ok((BlockSummary::leaf(laid.leaf)?, laid.layout))
    }

    fn answer(c: &Container, run: &BlockSummary) -> Result<LayoutOutput, LaneDecline> {
        lane_answer(c, run)
    }

    fn lanes(lanes: &mut Lanes) -> &mut FxHashMap<usize, Lane<Self>> {
        &mut lanes.block
    }
}

/// The container's inputs every child's leaf depends on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Share {
    inner_width: f32,
    item_basis_width: Option<f32>,
    content_box_inset: Rect<f32>,
    resolved_inset: Rect<f32>,
    resolved_border: Rect<f32>,
    outer_width: f32,
    direction: Direction,
    text_align: TextAlign,
    justify_items: AlignItems,
    own_margins_collapse: Line<bool>,
    children_adjoin: Line<bool>,
    scrolls: bool,
}

/// The heights children's percentages resolve against: their
/// `parent_size.height` and their sizes' basis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Heights {
    parent: Option<f32>,
    item_basis: Option<f32>,
}

impl Container {
    /// What every child's leaf depends on but the height bases.
    pub(crate) fn share(&self) -> Share {
        Share {
            inner_width: self.inner_width,
            item_basis_width: self.item_basis.width,
            content_box_inset: self.content_box_inset,
            resolved_inset: self.resolved_inset,
            resolved_border: self.resolved_border,
            outer_width: self.outer_width,
            direction: self.direction,
            text_align: self.text_align,
            justify_items: self.justify_items,
            own_margins_collapse: self.own_margins_collapse,
            children_adjoin: self.children_adjoin,
            scrolls: self.scrolls,
        }
    }

    /// The height bases.
    pub(crate) fn heights(&self) -> Heights {
        Heights {
            parent: self.parent_size.height,
            item_basis: self.item_basis.height,
        }
    }
}

/// Whether a child's leaf reads the height bases: a percentage (or a
/// calc, which may hold one) in its `height`, `min-height` or
/// `max-height` (`lane_block_item.rs`), or in its `top` or `bottom`
/// when it is `position: relative` (`lane_block_child_place.rs`). The
/// container's `parent_size.width` is its inner width, in the share.
pub(crate) fn reads_heights(style: &Style) -> bool {
    use taffy::{CompactLength, CoreStyle};
    let percent = |raw: CompactLength| raw.tag() == CompactLength::PERCENT_TAG || raw.has_calc();
    let inset = style.inset();
    let relative = style.position() == taffy::Position::Relative;
    let (min, max) = (style.min_size(), style.max_size());
    [
        style.size().height.into_raw(),
        min.height.into_raw(),
        max.height.into_raw(),
    ]
    .into_iter()
    .any(percent)
        || relative && (percent(inset.top.into_raw()) || percent(inset.bottom.into_raw()))
}
