//! A lane column's size from its own style, and the size query it
//! answers from that alone, mirrored from taffy 0.15.1
//! `compute/flexbox.rs` (`compute_flexbox_layout` 260-325);
//! `lane_block.rs` carries Taffy's MIT notice.

use taffy::util::{MaybeMath, MaybeResolve, ResolveOrZero};
use taffy::{
    BoxSizing, CoreStyle, LayoutInput, LayoutOutput, RequestedAxis, RunMode, Size, SizingMode,
    Style,
};

/// The column's size from its style and what it was given
/// (`styled_based_known_dimensions`, 260-301).
pub(super) fn styled_sizes(
    style: &Style,
    inputs: &LayoutInput,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Size<Option<f32>> {
    let (min_size, max_size) = limits(style, inputs.parent_size, calc);
    let clamped_style_size = if inputs.sizing_mode == SizingMode::InherentSize {
        let adjustment = box_sizing_adjustment(style, inputs.parent_size.width, calc);
        style
            .size()
            .maybe_resolve(inputs.parent_size, calc)
            .maybe_apply_aspect_ratio(style.aspect_ratio())
            .maybe_add(adjustment)
            .maybe_clamp(min_size, max_size)
    } else {
        Size::NONE
    };
    let min_max_definite_size = min_size.zip_map(max_size, |min, max| match (min, max) {
        (Some(min), Some(max)) if max <= min => Some(min),
        _ => None,
    });
    let padding_border = padding_border_sum(style, inputs.parent_size.width, calc);
    inputs.known_dimensions.or(min_max_definite_size
        .or(clamped_style_size)
        .maybe_max(padding_border))
}

/// A size query answered without the items (303-325): both sizes
/// known, or the width known and only the width asked.
pub(super) fn short_answer(inputs: &LayoutInput, known: Size<Option<f32>>) -> Option<LayoutOutput> {
    if inputs.run_mode != RunMode::ComputeSize {
        return None;
    }
    match (known.width, known.height) {
        (Some(width), Some(height)) => Some(LayoutOutput::from_outer_size(Size { width, height })),
        (Some(width), None) if inputs.axis == RequestedAxis::Horizontal => {
            Some(LayoutOutput::from_outer_size(Size { width, height: 0.0 }))
        }
        _ => None,
    }
}

/// `min-size` and `max-size` against the parent's size, with the
/// aspect ratio and box sizing applied (270-279, 719-728).
pub(super) fn limits(
    style: &Style,
    parent_size: Size<Option<f32>>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> (Size<Option<f32>>, Size<Option<f32>>) {
    let adjustment = box_sizing_adjustment(style, parent_size.width, calc);
    let ratio = style.aspect_ratio();
    let min = style.min_size().maybe_resolve(parent_size, calc);
    let max = style.max_size().maybe_resolve(parent_size, calc);
    (
        min.maybe_apply_aspect_ratio(ratio).maybe_add(adjustment),
        max.maybe_apply_aspect_ratio(ratio).maybe_add(adjustment),
    )
}

fn padding_border_sum(
    style: &Style,
    parent_width: Option<f32>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Size<f32> {
    let padding = style.padding().resolve_or_zero(parent_width, calc);
    let border = style.border().resolve_or_zero(parent_width, calc);
    padding.sum_axes() + border.sum_axes()
}

fn box_sizing_adjustment(
    style: &Style,
    parent_width: Option<f32>,
    calc: impl Fn(*const (), f32) -> f32 + Copy,
) -> Size<f32> {
    if style.box_sizing() == BoxSizing::ContentBox {
        padding_border_sum(style, parent_width, calc)
    } else {
        Size::ZERO
    }
}
