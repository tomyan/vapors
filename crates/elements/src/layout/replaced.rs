//! The size of a replaced box (an `img`, F2c S10) from its natural size,
//! what layout already knows, and its min and max lengths: CSS 2.1
//! section 10.4's table for replaced elements, in content-box pixels.
//! Taffy 0.15's own aspect-ratio handling applies the ratio to the
//! border box and before max-width clamps, and ignores the table, so the
//! natural ratio is applied here instead.

use taffy::{BoxSizing, Style};

/// A box's min and max content lengths, CSS px; 0 and infinity where
/// none is given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Limits {
    pub(crate) min: [f32; 2],
    pub(crate) max: [f32; 2],
}

/// The content size of a replaced box styled `style`, of `natural`
/// size, in a parent of `parent` size, given what layout already
/// `known`s (border-box sizes, as Taffy's leaf passes them). Taffy's leaf
/// layout does not pass the box's own width and height to its measure,
/// so they are resolved here.
pub(crate) fn measure(
    style: &Style,
    parent: taffy::Size<Option<f32>>,
    known: taffy::Size<Option<f32>>,
    natural: [f32; 2],
) -> taffy::Size<f32> {
    let edges = edges(style);
    let limits = limits(style, edges);
    let inset = inset(style);
    let content =
        |outer: Option<f32>, axis: usize| outer.map(|outer| (outer - inset[axis]).max(0.0));
    let own = |dimension, base, axis| {
        own(
            dimension,
            base,
            edges[axis],
            [limits.min[axis], limits.max[axis]],
        )
    };
    let known = taffy::Size {
        width: content(known.width, 0).or_else(|| own(style.size.width, parent.width, 0)),
        height: content(known.height, 1).or_else(|| own(style.size.height, parent.height, 1)),
    };
    size(known, natural, limits)
}

/// A box's own width or height in content px: a length, or a percentage
/// of `base`, less `edges` and within `[min, max]`; `None` for `auto`.
fn own(
    dimension: taffy::Dimension,
    base: Option<f32>,
    edges: f32,
    [min, max]: [f32; 2],
) -> Option<f32> {
    let raw = dimension.into_raw();
    let outer = match raw.tag() {
        taffy::CompactLength::LENGTH_TAG => Some(raw.value()),
        taffy::CompactLength::PERCENT_TAG => base.map(|base| base * raw.value()),
        _ => None,
    }?;
    Some((outer - edges).max(0.0).min(max).max(min))
}

fn length(raw: taffy::CompactLength) -> Option<f32> {
    (raw.tag() == taffy::CompactLength::LENGTH_TAG).then(|| raw.value())
}

/// What `style`'s lengths include beyond the content box: padding and
/// border under `box-sizing: border-box`, else nothing.
fn edges(style: &Style) -> [f32; 2] {
    if style.box_sizing == BoxSizing::BorderBox {
        inset(style)
    } else {
        [0.0, 0.0]
    }
}

/// Padding and border on each axis.
pub(super) fn inset(style: &Style) -> [f32; 2] {
    let side = |a: taffy::LengthPercentage, b: taffy::LengthPercentage| {
        length(a.into_raw()).unwrap_or(0.0) + length(b.into_raw()).unwrap_or(0.0)
    };
    [
        side(style.padding.left, style.padding.right) + side(style.border.left, style.border.right),
        side(style.padding.top, style.padding.bottom) + side(style.border.top, style.border.bottom),
    ]
}

/// The limits `style` gives, in content-box terms. Percentages are
/// refused on an `img` before layout (`images.rs`), so only lengths are
/// read here.
fn limits(style: &Style, edges: [f32; 2]) -> Limits {
    let min =
        [style.min_size.width, style.min_size.height].map(|d| length(d.into_raw()).unwrap_or(0.0));
    let max = [style.max_size.width, style.max_size.height]
        .map(|d| length(d.into_raw()).unwrap_or(f32::INFINITY));
    Limits {
        min: [0, 1].map(|axis| (min[axis] - edges[axis]).max(0.0)),
        max: [0, 1].map(|axis| (max[axis] - edges[axis]).max(0.0)),
    }
}

/// The content size of a replaced box of `natural` size, given what
/// layout already `known`s and the `limits`.
pub(crate) fn size(
    known: taffy::Size<Option<f32>>,
    [width, height]: [f32; 2],
    limits: Limits,
) -> taffy::Size<f32> {
    let clamp = |value: f32, axis: usize| value.min(limits.max[axis]).max(limits.min[axis]);
    let ratio = |a: f32, b: f32| if b > 0.0 { a / b } else { 0.0 };
    let [w, h] = match (known.width, known.height) {
        (Some(w), Some(h)) => [w, h],
        (Some(w), None) => [w, clamp(w * ratio(height, width), 1)],
        (None, Some(h)) => [clamp(h * ratio(width, height), 0), h],
        (None, None) => table([width, height], limits),
    };
    taffy::Size {
        width: w,
        height: h,
    }
}

/// CSS 2.1 10.4's table for both lengths `auto`.
fn table([w, h]: [f32; 2], limits: Limits) -> [f32; 2] {
    let max = [0, 1].map(|axis| limits.max[axis].max(limits.min[axis]));
    let [min_w, min_h] = limits.min;
    let [max_w, max_h] = max;
    let over = [w > max_w, h > max_h];
    let under = [w < min_w, h < min_h];
    let by = |scale: f32| [w * scale, h * scale];
    match (over, under) {
        ([true, true], _) if max_w / w <= max_h / h => [max_w, (max_w * h / w).max(min_h)],
        ([true, true], _) => [(max_h * w / h).max(min_w), max_h],
        (_, [true, true]) if min_w / w <= min_h / h => [(min_h * w / h).min(max_w), min_h],
        (_, [true, true]) => [min_w, (min_w * h / w).min(max_h)],
        ([false, true], [true, false]) => [min_w, max_h],
        ([true, false], [false, true]) => [max_w, min_h],
        ([true, false], _) => [max_w, (max_w * h / w).max(min_h)],
        ([false, true], _) => [(max_h * w / h).max(min_w), max_h],
        (_, [true, false]) => [min_w, (min_w * h / w).min(max_h)],
        (_, [false, true]) => [(min_h * w / h).min(max_w), min_h],
        _ => by(1.0),
    }
}

#[cfg(test)]
#[path = "replaced_tests.rs"]
mod tests;
