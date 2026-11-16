//! The flex lane (`lane_flex*.rs`) as a lane kind: what a column's
//! items' leaves depend on, split into the height inputs and the rest
//! as the block lane's are (`lane_share.rs`), since a flex parent asks
//! a column with its height unknown and then lays it out with it known.

use rustc_hash::FxHashMap;
use taffy::{
    AlignItems, AvailableSpace, Direction, Layout, LayoutOutput, NodeId, Point, Rect, RunMode, Size,
};

use super::lane::Lane;
use super::lane_flex::Column;
use super::lane_flex_answer::lane_flex_answer;
use super::lane_flex_summary::FlexSummary;
use super::lane_kind::LaneKind;
use super::lane_set::Lanes;
use super::lane_share::reads_heights;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// The flex lane.
#[derive(Debug)]
pub(crate) struct FlexLane;

/// The column's inputs every item's leaf depends on: all of `Column`
/// but its run mode, the requested axis and its height inputs. The
/// space along the column is not here either: a leaf holds the item's
/// shares under every kind of it (`lane_flex_contribution.rs`), its
/// height is measured under both min- and max-content space and the
/// item refused where they differ (`lane_flex_base.rs`), and it is
/// asked with a definite space along only when its height is known
/// (`lane_flex_child.rs`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FlexShare {
    direction: Direction,
    min_size: Size<Option<f32>>,
    max_size: Size<Option<f32>>,
    border: Rect<f32>,
    content_box_inset: Rect<f32>,
    scrollbar_gutter: Point<f32>,
    gap: f32,
    align_items: AlignItems,
    outer_width: Option<f32>,
    inner_width: Option<f32>,
    definite_width: bool,
    outer_space_width_is_definite: bool,
    space_across: AvailableSpace,
}

/// The column's height inputs: its outer and inner height when given,
/// and whether that height is definite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FlexHeights {
    outer: Option<f32>,
    inner: Option<f32>,
    definite: bool,
}

impl LaneKind for FlexLane {
    type Summary = FlexSummary;
    type Container = Column;
    type Share = FlexShare;
    type Heights = FlexHeights;

    fn share(c: &Column) -> FlexShare {
        FlexShare {
            direction: c.direction,
            min_size: c.min_size,
            max_size: c.max_size,
            border: c.border,
            content_box_inset: c.content_box_inset,
            scrollbar_gutter: c.scrollbar_gutter,
            gap: c.gap,
            align_items: c.align_items,
            outer_width: c.node_outer_size.width,
            inner_width: c.node_inner_size.width,
            definite_width: c.definite.width,
            outer_space_width_is_definite: c.outer_space_width_is_definite,
            space_across: c.available_space.width,
        }
    }

    fn heights(c: &Column) -> FlexHeights {
        FlexHeights {
            outer: c.node_outer_size.height,
            inner: c.node_inner_size.height,
            definite: c.definite.height,
        }
    }

    fn gap(c: &Column) -> f64 {
        f64::from(c.gap)
    }

    fn writes(c: &Column) -> bool {
        c.run_mode == RunMode::PerformLayout
    }

    fn top(c: &Column, _: &FlexSummary) -> f64 {
        f64::from(c.content_box_inset.top)
    }

    fn lay_out_child(
        tree: &mut BoxTree<'_>,
        slot: usize,
        c: &Column,
    ) -> Result<(FlexSummary, Layout), LaneDecline> {
        let laid = tree.lane_item(NodeId::from(slot), c)?;
        Ok((FlexSummary::leaf(laid.leaf), laid.layout))
    }

    fn answer(c: &Column, run: &FlexSummary) -> Result<LayoutOutput, LaneDecline> {
        lane_flex_answer(c, run)
    }

    /// The block lane's readers (`lane_share::reads_heights`), and an
    /// item whose `flex-basis` is relative: it resolves against the
    /// column's inner height when that is definite (`lane_flex_base.rs`,
    /// `styled_basis`).
    fn reads_heights(style: &taffy::Style) -> bool {
        use taffy::{CompactLength, FlexboxItemStyle};
        let basis = style.flex_basis().into_raw();
        reads_heights(style) || basis.tag() == CompactLength::PERCENT_TAG || basis.has_calc()
    }

    fn lanes(lanes: &mut Lanes) -> &mut FxHashMap<usize, Lane<Self>> {
        &mut lanes.flex
    }
}
