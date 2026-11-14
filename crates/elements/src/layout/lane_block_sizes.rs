//! The block lane container's sizes and short answers, mirrored from
//! taffy 0.15.1 `compute/block.rs` (`compute_block_layout` 384-446,
//! `compute_inner` 500-557, 605-612); `lane_block.rs` carries Taffy's
//! MIT notice.

use taffy::util::{MaybeMath, MaybeResolve};
use taffy::{
    AlignContentKeyword, BoxSizing, CoreStyle, Direction, LayoutInput, LayoutOutput, Line,
    Overflow, Rect, RequestedAxis, RunMode, Size, SizingMode,
};

/// A style's sizes resolved against the parent's size, aspect ratio
/// and box sizing applied (`compute_block_layout` 394-420).
pub(crate) struct Sizes {
    pub(super) size: Size<Option<f32>>,
    pub(super) min: Size<Option<f32>>,
    pub(super) max: Size<Option<f32>>,
    aspect_ratio: Option<f32>,
}

pub(super) fn sizes(
    style: &taffy::Style,
    parent_size: Size<Option<f32>>,
    padding_border_size: Size<f32>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Sizes {
    let aspect_ratio = style.aspect_ratio();
    let adjustment = if style.box_sizing() == BoxSizing::ContentBox {
        padding_border_size
    } else {
        Size::ZERO
    };
    let against = (parent_size, aspect_ratio, adjustment);
    Sizes {
        size: resolved(style.size(), against, calc),
        min: resolved(style.min_size(), against, calc),
        max: resolved(style.max_size(), against, calc),
        aspect_ratio,
    }
}

/// `size` resolved against a basis, with the aspect ratio applied and
/// the box-sizing adjustment added (`compute_block_layout` 401-416).
pub(super) fn resolved<D>(
    size: Size<D>,
    (basis, aspect_ratio, adjustment): (Size<Option<f32>>, Option<f32>, Size<f32>),
    calc: impl Fn(*const (), f32) -> f32,
) -> Size<Option<f32>>
where
    Size<D>: MaybeResolve<Size<Option<f32>>, Size<Option<f32>>>,
{
    size.maybe_resolve(basis, calc)
        .maybe_apply_aspect_ratio(aspect_ratio)
        .maybe_add(adjustment)
}

/// The container's known dimensions: as asked, else from a min above
/// its max, else from its clamped style size, each at least its padding
/// and border; the aspect ratio then fills a missing axis
/// (`compute_block_layout` 411-431, `compute_inner` 543-546).
pub(super) fn known_dimensions(
    inputs: &LayoutInput,
    sizes: &Sizes,
    padding_border_size: Size<f32>,
) -> Size<Option<f32>> {
    let clamped = if inputs.sizing_mode == SizingMode::InherentSize {
        sizes.size.maybe_clamp(sizes.min, sizes.max)
    } else {
        Size::NONE
    };
    let min_max_definite = sizes.min.zip_map(sizes.max, |min, max| match (min, max) {
        (Some(min), Some(max)) if max <= min => Some(min),
        _ => None,
    });
    let known = inputs
        .known_dimensions
        .or(min_max_definite)
        .or(clamped)
        .maybe_max(padding_border_size);
    let derived = known
        .maybe_apply_aspect_ratio(sizes.aspect_ratio)
        .maybe_clamp(sizes.min, sizes.max);
    Size {
        width: known.width.or(derived.width),
        height: known.height.or(derived.height),
    }
}

/// A `ComputeSize` ask answered from the known dimensions alone
/// (`compute_block_layout` 435-446, `compute_inner` 605-612).
pub(super) fn short_answer(inputs: &LayoutInput, known: Size<Option<f32>>) -> Option<LayoutOutput> {
    if inputs.run_mode != RunMode::ComputeSize {
        return None;
    }
    match (known.width, known.height, inputs.axis) {
        (Some(width), Some(height), _) => {
            Some(LayoutOutput::from_outer_size(Size { width, height }))
        }
        (Some(width), None, RequestedAxis::Horizontal) => {
            Some(LayoutOutput::from_outer_size(Size { width, height: 0.0 }))
        }
        _ => None,
    }
}

pub(super) fn is_scroll_container(style: &taffy::Style) -> bool {
    let overflow = style.overflow();
    overflow.x.is_scroll_container() || overflow.y.is_scroll_container()
}

pub(super) fn establishes_new_bfc(style: &taffy::Style) -> bool {
    is_scroll_container(style)
        || style.align_content.keyword != AlignContentKeyword::Normal
        || style.contain().establishes_independent_formatting_context()
}

/// The gutter `overflow: scroll` reserves, on the other axis
/// (`compute_inner` 500-509).
pub(super) fn scrollbar_gutter(style: &taffy::Style) -> Rect<f32> {
    let offsets = style.overflow().transpose().map(|overflow| match overflow {
        Overflow::Scroll => style.scrollbar_width(),
        _ => 0.0,
    });
    match style.direction() {
        Direction::Ltr => Rect {
            top: 0.0,
            left: 0.0,
            right: offsets.x,
            bottom: offsets.y,
        },
        Direction::Rtl => Rect {
            top: 0.0,
            left: offsets.x,
            right: 0.0,
            bottom: offsets.y,
        },
    }
}

/// Whether children's margins adjoin the container's edges, and
/// whether its own margins collapse with theirs (`compute_inner`
/// 563-578): neither through a new formatting context, an out-of-flow
/// box, padding or border; its bottom only with an auto height.
pub(super) fn collapsing(
    style: &taffy::Style,
    inputs: &LayoutInput,
    (padding, border): (Rect<f32>, Rect<f32>),
    height_is_auto: bool,
) -> (Line<bool>, Line<bool>) {
    let blocked = establishes_new_bfc(style) || style.position().is_out_of_flow();
    let adjoin = |collapsible: bool, padding: f32, border: f32| {
        collapsible && !blocked && padding == 0.0 && border == 0.0
    };
    let collapsible = inputs.vertical_margins_are_collapsible;
    let children_adjoin = Line {
        start: adjoin(collapsible.start, padding.top, border.top),
        end: adjoin(collapsible.end, padding.bottom, border.bottom),
    };
    let own = Line {
        start: children_adjoin.start,
        end: children_adjoin.end && height_is_auto,
    };
    (children_adjoin, own)
}
