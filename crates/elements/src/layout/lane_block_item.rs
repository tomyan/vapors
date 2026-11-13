//! A block lane child's style as the block algorithm reads it, mirrored
//! from taffy 0.15.1 `compute/block.rs` (`generate_item_list` 885-958,
//! `perform_final_layout_on_in_flow_children` 1372-1379);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::ResolveOrZero;
use taffy::{
    AlignContentKeyword, AlignItems, AlignItemsKeyword, BlockItemStyle, BoxSizing, Contain,
    CoreStyle, LengthPercentageAuto, NodeId, Overflow, Point, Position, Rect, Size,
};

use super::lane_block::Container;
use super::lane_block_sizes::resolved;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// The child's style as the block algorithm reads it.
pub(super) struct Item {
    pub(super) same_bfc: bool,
    pub(super) relative: bool,
    pub(super) compressible_replaced: bool,
    pub(super) size: Size<Option<f32>>,
    pub(super) min: Size<Option<f32>>,
    pub(super) max: Size<Option<f32>>,
    pub(super) overflow: Point<Overflow>,
    pub(super) contain: Contain,
    pub(super) scrollbar_width: f32,
    /// `justify-self` resolved against the container's `justify-items`.
    pub(super) justify_self: AlignItems,
    pub(super) inset: Rect<LengthPercentageAuto>,
    /// Margins resolved against the container's inner width; `None`
    /// for `auto`.
    pub(super) margin: Rect<Option<f32>>,
    pub(super) padding: Rect<f32>,
    pub(super) border: Rect<f32>,
    pub(super) padding_border_sum: Size<f32>,
}

impl BoxTree<'_> {
    /// The child's style read as `generate_item_list` reads it
    /// (885-958), with the container's `justify-items` resolved
    /// (1372-1379).
    pub(super) fn item(&self, id: NodeId, c: &Container) -> Result<Item, LaneDecline> {
        let style = &self.boxes[usize::from(id)].style;
        let calc = |val, basis| self.calcs.resolve(val, basis);
        if style.position().is_out_of_flow() {
            return Err(LaneDecline::OutOfFlowChild);
        }
        let keyword = |raw: taffy::CompactLength| raw.is_sizing_keyword();
        let size_style = style.size();
        if [size_style.width.into_raw(), size_style.height.into_raw()]
            .into_iter()
            .chain(
                [style.min_size(), style.max_size()]
                    .iter()
                    .flat_map(|s| [s.width.into_raw(), s.height.into_raw()]),
            )
            .any(keyword)
        {
            return Err(LaneDecline::SizingKeyword);
        }
        let justify_self = style
            .justify_self()
            .unwrap_or(c.justify_items)
            .resolve_self_relative(style.direction(), c.direction, true);
        let stretched = matches!(
            justify_self.keyword,
            AlignItemsKeyword::Normal | AlignItemsKeyword::Stretch
        );
        if !stretched && style.size().width.is_auto() {
            return Err(LaneDecline::JustifySelfFitContent);
        }
        let aspect_ratio = style.aspect_ratio();
        let padding = style.padding().resolve_or_zero(c.item_basis.width, calc);
        let border = style.border().resolve_or_zero(c.item_basis.width, calc);
        let padding_border_sum = (padding + border).sum_axes();
        let adjustment = if style.box_sizing() == BoxSizing::ContentBox {
            padding_border_sum
        } else {
            Size::ZERO
        };
        let against = (c.item_basis, aspect_ratio, adjustment);
        let overflow = style.overflow();
        let contain = style.contain();
        let same_bfc = style.is_block()
            && !style.is_table()
            && !overflow.x.is_scroll_container()
            && !overflow.y.is_scroll_container()
            && !contain.establishes_independent_formatting_context()
            && style.align_content.keyword == AlignContentKeyword::Normal;
        Ok(Item {
            same_bfc,
            relative: style.position() == Position::Relative,
            compressible_replaced: style.is_compressible_replaced(),
            size: resolved(size_style, against, calc),
            min: resolved(style.min_size(), against, calc),
            max: resolved(style.max_size(), against, calc),
            overflow,
            contain,
            scrollbar_width: style.scrollbar_width(),
            justify_self,
            inset: style.inset(),
            margin: style
                .margin()
                .map(|margin| margin.resolve_to_option(c.inner_width, calc)),
            padding,
            border,
            padding_border_sum,
        })
    }
}
