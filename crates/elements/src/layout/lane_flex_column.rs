//! A lane column's own inputs, mirrored from taffy 0.15.1
//! `compute/flexbox.rs` (`compute_flexbox_layout` 255-348,
//! `compute_constants` 634-749, `determine_available_space` 896-945);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::{MaybeMath, ResolveOrZero};
use taffy::{
    AlignContentKeyword, AlignItems, AlignItemsKeyword, AvailableSpace, CompactLength, CoreStyle,
    Direction, FlexDirection, FlexWrap, FlexboxContainerStyle, LayoutInput, Overflow, Point, Size,
    Style,
};

use super::lane_flex::Column;
use super::lane_flex_sizes::limits;
use crate::lane_decline::LaneDecline;

/// The column's own style refused before it is asked anything: not a
/// single-line column packed from its start (section 10.3).
pub(super) fn column_declines(style: &Style) -> Result<(), LaneDecline> {
    match style.flex_direction() {
        FlexDirection::Column => {}
        FlexDirection::ColumnReverse => return Err(LaneDecline::ColumnReverse),
        FlexDirection::Row | FlexDirection::RowReverse => return Err(LaneDecline::FlexRow),
    }
    if style.flex_wrap() != FlexWrap::NoWrap {
        return Err(LaneDecline::FlexWrap);
    }
    if !matches!(
        style.justify_content().keyword,
        AlignContentKeyword::Normal
            | AlignContentKeyword::FlexStart
            | AlignContentKeyword::Start
            | AlignContentKeyword::Stretch
    ) {
        return Err(LaneDecline::JustifyContent);
    }
    Ok(())
}

/// The column's constants (`compute_constants` 634-749), with the
/// definiteness of what it was given normalised (331-333).
///
/// # Errors
/// A percentage `row-gap` while the height is not known
/// (`RelativeGap`): Taffy resolves it again against the height the
/// items sum to (`compute_preliminary` 428-438).
pub(super) fn constants(
    style: &Style,
    inputs: &LayoutInput,
    known: Size<Option<f32>>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Result<Column, LaneDecline> {
    let definite = inputs
        .known_dimensions_are_definite
        .zip_map(inputs.known_dimensions, |is, given| is || given.is_none());
    let content_box_inset = content_box_inset(style, inputs.parent_size.width, calc);
    let node_inner_size = known.maybe_sub(content_box_inset.sum_axes());
    let gap = row_gap(style, node_inner_size.height, calc)?;
    let (min_size, max_size) = limits(style, inputs.parent_size, calc);
    Ok(Column {
        run_mode: inputs.run_mode,
        axis: inputs.axis,
        direction: style.direction(),
        min_size,
        max_size,
        border: style
            .border()
            .resolve_or_zero(inputs.parent_size.width, calc),
        content_box_inset,
        scrollbar_gutter: scrollbar_gutter(style),
        scrolls: style.overflow().x.is_scroll_container()
            || style.overflow().y.is_scroll_container(),
        gap,
        align_items: normal_to_stretch(style.align_items()),
        node_outer_size: known,
        node_inner_size,
        definite,
        outer_space_width_is_definite: matches!(
            inputs.available_space.width,
            AvailableSpace::Definite(_)
        ),
        available_space: available_space(known, inputs.available_space, content_box_inset),
        suppresses_baseline: style.contain().suppresses_baseline(),
    })
}

/// `align-items: normal` is `stretch` in a flex container (625-630).
pub(super) fn normal_to_stretch(alignment: AlignItems) -> AlignItems {
    match alignment.keyword {
        AlignItemsKeyword::Normal => AlignItems::STRETCH,
        _ => alignment,
    }
}

/// The column's `row-gap`, against its inner height when known, else
/// against 0 (703); a percentage then is refused.
fn row_gap(
    style: &Style,
    inner_height: Option<f32>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Result<f32, LaneDecline> {
    let raw = style.gap().height.into_raw();
    let relative = raw.tag() == CompactLength::PERCENT_TAG || raw.has_calc();
    if relative && inner_height.is_none() {
        return Err(LaneDecline::RelativeGap);
    }
    Ok(style
        .gap()
        .height
        .resolve_or_zero(inner_height.or(Some(0.0)), calc))
}

/// A scrollbar's width where the other axis scrolls (679-682).
fn scrollbar_gutter(style: &Style) -> Point<f32> {
    style.overflow().transpose().map(|overflow| match overflow {
        Overflow::Scroll => style.scrollbar_width(),
        _ => 0.0,
    })
}

/// Padding, border and the scrollbar gutter (688-694).
fn content_box_inset(
    style: &Style,
    parent_width: Option<f32>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> taffy::Rect<f32> {
    let gutter = scrollbar_gutter(style);
    let mut inset = style.padding().resolve_or_zero(parent_width, calc)
        + style.border().resolve_or_zero(parent_width, calc);
    inset.bottom += gutter.y;
    match style.direction() {
        Direction::Ltr => inset.right += gutter.x,
        Direction::Rtl => inset.left += gutter.x,
    }
    inset
}

/// The space inside the column (`determine_available_space` 896-945).
fn available_space(
    known: Size<Option<f32>>,
    outer: Size<AvailableSpace>,
    inset: taffy::Rect<f32>,
) -> Size<AvailableSpace> {
    let inner = |known: Option<f32>, outer: AvailableSpace, sum: f32| match known {
        Some(size) => AvailableSpace::Definite((size - sum).max(0.0)),
        None => outer.maybe_sub(sum).maybe_max(0.0),
    };
    Size {
        width: inner(known.width, outer.width, inset.horizontal_axis_sum()),
        height: inner(known.height, outer.height, inset.vertical_axis_sum()),
    }
}
