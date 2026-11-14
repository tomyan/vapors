//! A block lane child's style as the block algorithm reads it, mirrored
//! from taffy 0.15.1 `compute/block.rs` (`generate_item_list` 885-958,
//! `perform_final_layout_on_in_flow_children` 1372-1392);
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

/// The child's style as the block algorithm reads it. Taffy also reads
/// `is_table` for the same formatting context (912-918) and for
/// stretching (1383); vapors has no table display (`dispatch` refuses
/// it), so neither is mirrored.
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
    /// (1372-1392).
    pub(super) fn item(&self, id: NodeId, c: &Container) -> Result<Item, LaneDecline> {
        let style = &self.boxes[usize::from(id)].style;
        let calc = |val, basis| self.calcs.resolve(val, basis);
        let justify_self = justify_self(style, c)?;
        let padding = style.padding().resolve_or_zero(c.item_basis.width, calc);
        let border = style.border().resolve_or_zero(c.item_basis.width, calc);
        let padding_border_sum = (padding + border).sum_axes();
        let adjustment = if style.box_sizing() == BoxSizing::ContentBox {
            padding_border_sum
        } else {
            Size::ZERO
        };
        let against = (c.item_basis, style.aspect_ratio(), adjustment);
        Ok(Item {
            same_bfc: same_bfc(style),
            relative: style.position() == Position::Relative,
            compressible_replaced: style.is_compressible_replaced(),
            size: resolved(style.size(), against, calc),
            min: resolved(style.min_size(), against, calc),
            max: resolved(style.max_size(), against, calc),
            overflow: style.overflow(),
            contain: style.contain(),
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

/// The child's `justify-self` against the container's `justify-items`
/// (1372-1379), with the cases a lane does not lay out refused: an
/// out-of-flow child, a sizing keyword, and `fit-content` from a
/// non-stretch `justify-self` on an auto width (1388-1392).
fn justify_self(style: &taffy::Style, c: &Container) -> Result<AlignItems, LaneDecline> {
    if style.position().is_out_of_flow() {
        return Err(LaneDecline::OutOfFlowChild);
    }
    let size = style.size();
    let limits = [style.min_size(), style.max_size()];
    let raw = [size.width.into_raw(), size.height.into_raw()]
        .into_iter()
        .chain(
            limits
                .iter()
                .flat_map(|s| [s.width.into_raw(), s.height.into_raw()]),
        );
    if raw.into_iter().any(taffy::CompactLength::is_sizing_keyword) {
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
    Ok(justify_self)
}

/// Whether the child is laid out in the container's block formatting
/// context (912-918).
fn same_bfc(style: &taffy::Style) -> bool {
    let overflow = style.overflow();
    style.is_block()
        && !overflow.x.is_scroll_container()
        && !overflow.y.is_scroll_container()
        && !style.contain().establishes_independent_formatting_context()
        && style.align_content.keyword == AlignContentKeyword::Normal
}
