//! A lane column's item as the flex algorithm reads its style,
//! mirrored from taffy 0.15.1 `compute/flexbox.rs`
//! (`generate_anonymous_flex_items` 777-895, `resolve_relative_inset`
//! 751-775), with the item cases a lane does not lay out refused
//! (`docs/plans/splice-layout.md`, section 10.3); `lane_block.rs`
//! carries Taffy's MIT notice.

use taffy::util::{MaybeMath, MaybeResolve, ResolveOrZero};
use taffy::{
    AlignItems, BoxSizing, Contain, CoreStyle, Dimension, FlexboxItemStyle, LengthPercentageAuto,
    NodeId, Overflow, Point, Position, Rect, Size, Style,
};

use super::lane_flex::Column;
use super::lane_flex_column::normal_to_stretch;
use super::tree::BoxTree;
use crate::lane_decline::LaneDecline;

/// The item's style as the flex algorithm reads it.
pub(super) struct FlexChild {
    pub(super) size: Size<Option<f32>>,
    pub(super) size_style: Size<Dimension>,
    pub(super) min_size: Size<Option<f32>>,
    pub(super) max_size: Size<Option<f32>>,
    pub(super) aspect_ratio: Option<f32>,
    pub(super) align_self: AlignItems,
    pub(super) overflow: Point<Overflow>,
    pub(super) contain: Contain,
    pub(super) scrollbar_width: f32,
    pub(super) flex_shrink: f32,
    pub(super) flex_basis_style: Dimension,
    /// Its `position: relative` offset: `x` across, `y` along.
    pub(super) relative_inset: Point<f32>,
    pub(super) margin: Rect<f32>,
    pub(super) margin_is_auto: Rect<bool>,
    pub(super) padding: Rect<f32>,
    pub(super) border: Rect<f32>,
    pub(super) box_sizing_adjustment: Size<f32>,
}

impl FlexChild {
    pub(super) fn is_scroll_container(&self) -> bool {
        self.overflow.x.is_scroll_container() || self.overflow.y.is_scroll_container()
    }

    /// Stretched across the column (`is_stretched`, 138-143).
    pub(super) fn is_stretched(&self) -> bool {
        let width = self.size_style.width;
        !self.margin_is_auto.left
            && !self.margin_is_auto.right
            && (width.is_stretch() || (self.align_self == AlignItems::STRETCH && width.is_auto()))
    }
}

impl BoxTree<'_> {
    /// The item at `id` read as `generate_anonymous_flex_items` reads
    /// it (787-892).
    ///
    /// # Errors
    /// An item the lane does not lay out (`refused`).
    pub(super) fn flex_child(&self, id: NodeId, column: &Column) -> Result<FlexChild, LaneDecline> {
        let style = &self.boxes[usize::from(id)].style;
        refused(style)?;
        let calc = |val, basis| self.calcs.resolve(val, basis);
        let inner = column.node_inner_size;
        let percent_basis = percent_basis(column);
        let padding = style.padding().resolve_or_zero(inner.width, calc);
        let border = style.border().resolve_or_zero(inner.width, calc);
        let adjustment = if style.box_sizing() == BoxSizing::ContentBox {
            (padding + border).sum_axes()
        } else {
            Size::ZERO
        };
        Ok(FlexChild {
            size: style
                .size()
                .maybe_resolve(percent_basis, calc)
                .maybe_apply_aspect_ratio(style.aspect_ratio())
                .maybe_add(adjustment),
            size_style: style.size(),
            min_size: style
                .min_size()
                .maybe_resolve(percent_basis, calc)
                .maybe_add(adjustment),
            max_size: style
                .max_size()
                .maybe_resolve(percent_basis, calc)
                .maybe_add(adjustment),
            aspect_ratio: style.aspect_ratio(),
            align_self: align_self(style, column),
            overflow: style.overflow(),
            contain: style.contain(),
            scrollbar_width: style.scrollbar_width(),
            flex_shrink: style.flex_shrink(),
            flex_basis_style: style.flex_basis(),
            relative_inset: self.relative_inset(style, column),
            margin: style.margin().resolve_or_zero(inner.width, calc),
            margin_is_auto: style.margin().map(LengthPercentageAuto::is_auto),
            padding,
            border,
            box_sizing_adjustment: adjustment,
        })
    }

    /// A relative item's offset (831-839, 751-775): `top` over
    /// `bottom` along; `left` over `right` across, the reverse in RTL.
    fn relative_inset(&self, style: &Style, column: &Column) -> Point<f32> {
        if style.position() != Position::Relative {
            return Point::ZERO;
        }
        let calc = |val, basis| self.calcs.resolve(val, basis);
        let (inset, inner) = (style.inset(), column.node_inner_size);
        let inset = Rect {
            left: inset.left.maybe_resolve(inner.width, calc),
            right: inset.right.maybe_resolve(inner.width, calc),
            top: inset.top.maybe_resolve(inner.height, calc),
            bottom: inset.bottom.maybe_resolve(inner.height, calc),
        };
        let y = inset.top.or(inset.bottom.map(|pos| -pos)).unwrap_or(0.0);
        let x = if column.direction.is_rtl() {
            inset.right.map(|pos| -pos).or(inset.left)
        } else {
            inset.left.or(inset.right.map(|pos| -pos))
        };
        Point {
            x: x.unwrap_or(0.0),
            y,
        }
    }
}

/// What an item's percentage sizes resolve against (785-789): the
/// column's inner size, its height only when definite.
fn percent_basis(column: &Column) -> Size<Option<f32>> {
    let inner = column.node_inner_size;
    if column.known_main_size_is_definite() {
        inner
    } else {
        Size {
            height: None,
            ..inner
        }
    }
}

/// The item's `align-self` against the column's `align-items`
/// (845-846).
fn align_self(style: &Style, column: &Column) -> AlignItems {
    normal_to_stretch(style.align_self().unwrap_or(column.align_items)).resolve_self_relative(
        style.direction(),
        column.direction,
        true,
    )
}

/// The items a lane column does not lay out: out of flow, growing,
/// with an `auto` margin along the column, or sized by a keyword. An
/// item aligned by `baseline` is placed by itself in a column (Taffy
/// reads line baselines only in rows, `calculate_children_base_lines`
/// 2054-2056).
fn refused(style: &Style) -> Result<(), LaneDecline> {
    if style.position().is_out_of_flow() {
        return Err(LaneDecline::OutOfFlowChild);
    }
    if style.flex_grow() > 0.0 {
        return Err(LaneDecline::FlexGrow);
    }
    let margin = style.margin();
    if margin.top.is_auto() || margin.bottom.is_auto() {
        return Err(LaneDecline::MainAutoMargin);
    }
    sizing_keywords(style)
}

/// A sizing keyword in the item's sizes or flex basis, which vapors'
/// styles never produce: not mirrored.
fn sizing_keywords(style: &Style) -> Result<(), LaneDecline> {
    let (size, min, max) = (style.size(), style.min_size(), style.max_size());
    let raw = [
        size.width.into_raw(),
        size.height.into_raw(),
        min.width.into_raw(),
        min.height.into_raw(),
        max.width.into_raw(),
        max.height.into_raw(),
        style.flex_basis().into_raw(),
    ];
    if raw.into_iter().any(taffy::CompactLength::is_sizing_keyword) {
        return Err(LaneDecline::SizingKeyword);
    }
    Ok(())
}
