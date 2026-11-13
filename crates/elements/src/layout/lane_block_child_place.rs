//! Where a block lane's child sits across the axis and what it
//! contributes to the container's overflow, mirrored from taffy 0.15.1
//! `compute/block.rs` (`resolve_relative_offset` 1007-1023,
//! `perform_final_layout_on_in_flow_children` 1588-1601, 1640-1692,
//! 1734-1752) and `compute/common/scrollable_overflow.rs` (25-70);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::MaybeResolve;
use taffy::{
    AlignItemsKeyword, AlignmentSafety, Direction, LayoutOutput, Overflow, Point, Rect, Size,
    TextAlign,
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
/// then aligned by `justify-self` or `text-align` (1640-1692): the
/// same in and out of the container's formatting context without
/// floats.
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
    let non_stretch = match item.justify_self.keyword {
        AlignItemsKeyword::Normal | AlignItemsKeyword::Stretch => None,
        _ => Some(item.justify_self),
    };
    match non_stretch {
        Some(justify) if !has_auto_x_margin => {
            let free = c.inner_width - outer_width;
            let keyword = if matches!(justify.safety, AlignmentSafety::Safe) && free < 0.0 {
                AlignItemsKeyword::Start
            } else {
                justify.keyword
            };
            let to_end = match keyword {
                AlignItemsKeyword::Center => None,
                AlignItemsKeyword::End
                | AlignItemsKeyword::FlexEnd
                | AlignItemsKeyword::SelfEnd => Some(true),
                _ => Some(false),
            };
            let offset = match to_end {
                None => free / 2.0,
                Some(end) if end != c.direction.is_rtl() => free,
                Some(_) => 0.0,
            };
            c.resolved_inset.left + margin.left + inset_x + offset
        }
        None if outer_width < c.inner_width => {
            start + text_align_shift(c, c.inner_width - outer_width)
        }
        _ => start,
    }
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
/// container's border): empty when its extent is empty. A child wholly
/// before a scroll container's origin is not left out: that changes
/// only the rect's top or left, which `Layout::scroll_width` and
/// `scroll_height` do not read.
pub(super) fn contribution(
    c: &Container,
    item: &Item,
    output: &LayoutOutput,
    placing: &Placing,
) -> ([f64; 2], [f64; 2]) {
    let rect = output.scrollable_overflow_rect;
    let scrolls = item.overflow.x.is_scroll_container() || item.overflow.y.is_scroll_container();
    let contained = item.contain.contains_scrollable_overflow();
    let propagates = |overflow: Overflow| !scrolls && !contained && overflow == Overflow::Visible;
    let (size, location) = (placing.size, placing.location);
    let end = Size {
        width: if propagates(item.overflow.x) {
            size.width.max(rect.right)
        } else {
            size.width
        },
        height: if propagates(item.overflow.y) {
            size.height.max(rect.bottom)
        } else {
            size.height
        },
    };
    if end.width <= 0.0 || end.height <= 0.0 {
        return (
            [f64::INFINITY, f64::NEG_INFINITY],
            [f64::INFINITY, f64::NEG_INFINITY],
        );
    }
    let start = Point {
        x: if propagates(item.overflow.x) {
            rect.left.min(0.0)
        } else {
            0.0
        },
        y: if propagates(item.overflow.y) {
            rect.top.min(0.0)
        } else {
            0.0
        },
    };
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
