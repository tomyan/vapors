//! Where a block lane's child sits across the axis and what it
//! contributes to the container's overflow, mirrored from taffy 0.15.1
//! `compute/block.rs` (`resolve_relative_offset` 1007-1023,
//! `perform_final_layout_on_in_flow_children` 1588-1601, 1640-1692,
//! 1734-1752) and `compute/common/scrollable_overflow.rs` (25-63);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeResolve;
use taffy::{
    AlignItemsKeyword, AlignmentSafety, Contain, Direction, LayoutOutput, Overflow, Point, Rect,
    Size, TextAlign,
};

use super::lane_block::Container;
use super::lane_block_item::Item;

/// A child's size and its location, with `location.y` relative to its
/// start along the axis.
pub(super) struct Placing {
    pub(super) size: Size<f32>,
    pub(super) location: Point<f32>,
}

/// A `position: relative` child's offset by its inset (CSS 2.2 9.4.3):
/// `top` over `bottom`, `left` over `right` in LTR and the reverse in
/// RTL; a percentage resolves against the container's content box.
pub(super) fn relative_offset(
    item: &Item,
    c: &Container,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Point<f32> {
    if !item.relative {
        return Point::ZERO;
    }
    let (width, height) = (c.parent_size.width, c.parent_size.height);
    let inset = Rect {
        left: item.inset.left.maybe_resolve(width, calc),
        right: item.inset.right.maybe_resolve(width, calc),
        top: item.inset.top.maybe_resolve(height, calc),
        bottom: item.inset.bottom.maybe_resolve(height, calc),
    };
    let x = if c.direction.is_rtl() {
        inset.right.map(|x| -x).or(inset.left)
    } else {
        inset.left.or(inset.right.map(|x| -x))
    };
    Point {
        x: x.unwrap_or(0.0),
        y: inset.top.or(inset.bottom.map(|y| -y)).unwrap_or(0.0),
    }
}

/// Where the child's border box starts across the axis (1588-1601),
/// then aligned by `justify-self` or `text-align` (1640-1692). Without
/// floats the start is the same in and out of the container's
/// formatting context, but for RTL out of it (1623): Taffy goes
/// through the inner width, clamped at 0, so the two differ when the
/// inset exceeds the outer width; vapors has no `direction` property,
/// so the same-context formula serves both.
pub(super) fn placed_x(
    c: &Container,
    item: &Item,
    (width, margin): (f32, Rect<f32>),
    inset_x: f32,
) -> f32 {
    let start = match c.direction {
        Direction::Ltr => c.resolved_inset.left + inset_x + margin.left,
        Direction::Rtl => c.outer_width - c.resolved_inset.right - width - margin.right + inset_x,
    };
    let outer_width = width + margin.left + margin.right;
    let has_auto_x_margin = item.margin.left.is_none() || item.margin.right.is_none();
    match item.justify_self.keyword {
        AlignItemsKeyword::Normal | AlignItemsKeyword::Stretch if outer_width < c.inner_width => {
            start + text_align_shift(c, c.inner_width - outer_width)
        }
        AlignItemsKeyword::Normal | AlignItemsKeyword::Stretch => start,
        _ if has_auto_x_margin => start,
        _ => justified_x(c, item, margin.left + inset_x, c.inner_width - outer_width),
    }
}

/// A non-stretch `justify-self` places the margin box in the free
/// space (1645-1674), `safe` falling back to the start when it
/// overflows.
fn justified_x(c: &Container, item: &Item, lead: f32, free: f32) -> f32 {
    let justify = item.justify_self;
    let keyword = if matches!(justify.safety, AlignmentSafety::Safe) && free < 0.0 {
        AlignItemsKeyword::Start
    } else {
        justify.keyword
    };
    let to_end = match keyword {
        AlignItemsKeyword::Center => None,
        AlignItemsKeyword::End | AlignItemsKeyword::FlexEnd | AlignItemsKeyword::SelfEnd => {
            Some(true)
        }
        _ => Some(false),
    };
    let offset = match to_end {
        None => free / 2.0,
        Some(end) if end != c.direction.is_rtl() => free,
        Some(_) => 0.0,
    };
    c.resolved_inset.left + lead + offset
}

/// Legacy `text-align` on a block container shifts a narrower child
/// (1677-1691).
fn text_align_shift(c: &Container, free: f32) -> f32 {
    match (c.text_align, c.direction) {
        (TextAlign::Auto, _)
        | (TextAlign::LegacyLeft, Direction::Ltr)
        | (TextAlign::LegacyRight, Direction::Rtl) => 0.0,
        (TextAlign::LegacyLeft, Direction::Rtl) => -free,
        (TextAlign::LegacyRight, Direction::Ltr) => free,
        (TextAlign::LegacyCenter, Direction::Ltr) => free / 2.0,
        (TextAlign::LegacyCenter, Direction::Rtl) => -free / 2.0,
    }
}

/// What the child contributes to the container's scrollable overflow,
/// from its border-box start along the axis (`[low, reach]`, the
/// relative offset included) and across it (in the container, from the
/// container's border): empty when its extent is empty. Whether a
/// scroll container leaves it out as wholly before the origin is
/// decided in the answer, which knows the origin
/// (`lane_block_answer.rs`).
pub(super) fn contribution(
    c: &Container,
    item: &Item,
    output: &LayoutOutput,
    placing: &Placing,
) -> ([f64; 2], [f64; 2]) {
    let rect = output.scrollable_overflow_rect;
    let propagates = propagates(item.overflow, item.contain);
    let end = end_extent(placing.size, rect, propagates);
    if end.width <= 0.0 || end.height <= 0.0 {
        return (
            [f64::INFINITY, f64::NEG_INFINITY],
            [f64::INFINITY, f64::NEG_INFINITY],
        );
    }
    let start = start_extent(rect, propagates);
    let (size, location) = (placing.size, placing.location);
    let x = if c.direction.is_rtl() {
        c.outer_width - (location.x + size.width) - c.resolved_border.right
    } else {
        location.x - c.resolved_border.left
    };
    let along = [
        f64::from(location.y + start.y),
        f64::from(location.y + end.height),
    ];
    let across = [f64::from(x + start.x), f64::from(x + end.width)];
    (along, across)
}

/// On which axes a child's own overflow reaches its parent (34-41).
pub(super) fn propagates(overflow: Point<Overflow>, contain: Contain) -> Point<bool> {
    let scrolls = overflow.x.is_scroll_container() || overflow.y.is_scroll_container();
    let contained = contain.contains_scrollable_overflow();
    let on = |axis: Overflow| !scrolls && !contained && axis == Overflow::Visible;
    Point {
        x: on(overflow.x),
        y: on(overflow.y),
    }
}

/// How far the child's box or overflow ends past its start (42-45).
pub(super) fn end_extent(size: Size<f32>, rect: Rect<f32>, propagates: Point<bool>) -> Size<f32> {
    Size {
        width: if propagates.x {
            size.width.max(rect.right)
        } else {
            size.width
        },
        height: if propagates.y {
            size.height.max(rect.bottom)
        } else {
            size.height
        },
    }
}

/// How far before its start the child's overflow begins (49-52).
pub(super) fn start_extent(rect: Rect<f32>, propagates: Point<bool>) -> Point<f32> {
    Point {
        x: if propagates.x {
            rect.left.min(0.0)
        } else {
            0.0
        },
        y: if propagates.y { rect.top.min(0.0) } else { 0.0 },
    }
}
