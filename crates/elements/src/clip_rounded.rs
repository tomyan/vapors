//! Rounded clips (flat-scene F3.2b, `docs/plans/flat-scene-f3.md` items
//! 13 and F3.2b): a box with `overflow: hidden` or `clip` and a corner
//! radius cuts its content to its padding box with the padding box's
//! radii. A rounded clip meets another clip only where one holds the
//! other (Tom, 2026-11-27); any other overlap is refused after paint
//! (`clip_refusals.rs`), naming clip chains, which would retire it.

use vapors_scene::Rect;

use crate::clip::{BoxClip, within};
use crate::layout::{BoxRect, LayoutBox};

/// What a box cuts its content to, met with the clip it is under.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Met {
    /// One clip holds the other, or both are square: the clip.
    Clip(BoxClip),
    /// A rounded clip and another overlap without either holding the
    /// other: refused; their rectangles' intersection, square, stands in
    /// until the refusal.
    Overlap(BoxClip),
}

/// The padding box's corner radii, CSS pixels, of `layout_box` at
/// `border_box` with border radii `radii`: the radii fitted to the
/// border box as CSS scales them (Backgrounds 3 §5.5), less the border
/// at each corner, at least 0 (§5.2). A corner's two borders are one
/// width: an elliptical inner corner is refused (`style/decoration.rs`).
pub(crate) fn padding_radii(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    radii: [f32; 4],
) -> [f32; 4] {
    let fitted = crate::corners::fitted(radii, rect(border_box));
    let border = layout_box.layout.border;
    let corners = [
        border.top.max(border.left),
        border.top.max(border.right),
        border.bottom.max(border.right),
        border.bottom.max(border.left),
    ];
    std::array::from_fn(|i| (fitted[i] - corners[i]).max(0.0))
}

/// `candidate` met with `own`, the clip the box is under; `radii_of`
/// gives the radii of the box a rounded clip names.
pub(crate) fn meet(
    own: Option<BoxClip>,
    candidate: BoxClip,
    radii_of: &dyn Fn(usize) -> [f32; 4],
) -> Met {
    let Some(own) = own else {
        return Met::Clip(candidate);
    };
    let radii = |clip: BoxClip| clip.rounded_by().map_or([0.0; 4], radii_of);
    if holds(own.rect, radii(own), candidate.rect) {
        return Met::Clip(candidate);
    }
    if holds(candidate.rect, radii(candidate), own.rect) {
        return Met::Clip(own);
    }
    let cut = BoxClip::square(within(Some(own.rect), candidate.rect));
    let square = own.rounded_by().is_none() && candidate.rounded_by().is_none();
    if square || cut.rect.width <= 0.0 || cut.rect.height <= 0.0 {
        Met::Clip(cut)
    } else {
        Met::Overlap(cut)
    }
}

/// Whether `inner` lies within `outer` with corner `radii`: inside its
/// rectangle, and each of its corners inside the circle of the corner
/// it is nearest. The rounded shape is convex, so its four corners in
/// it put the whole rectangle in it.
pub(crate) fn holds(outer: BoxRect, radii: [f32; 4], inner: BoxRect) -> bool {
    let (o, i) = (rect(outer), rect(inner));
    let inside = o.left <= i.left && o.top <= i.top && i.right <= o.right && i.bottom <= o.bottom;
    let points = [
        (i.left, i.top),
        (i.right, i.top),
        (i.right, i.bottom),
        (i.left, i.bottom),
    ];
    inside && (0..4).all(|corner| in_corner(o, radii[corner], corner, points[corner]))
}

/// Whether `point` is inside `rect`'s corner `corner` rounded by
/// `radius`: outside the corner's square, or within the circle in it.
fn in_corner(rect: Rect, radius: f32, corner: usize, (x, y): (f32, f32)) -> bool {
    if radius <= 0.0 {
        return true;
    }
    let cx = if corner == 0 || corner == 3 {
        rect.left + radius
    } else {
        rect.right - radius
    };
    let cy = if corner < 2 {
        rect.top + radius
    } else {
        rect.bottom - radius
    };
    let beyond_x = if corner == 0 || corner == 3 {
        x < cx
    } else {
        x > cx
    };
    let beyond_y = if corner < 2 { y < cy } else { y > cy };
    !(beyond_x && beyond_y) || (x - cx).hypot(y - cy) <= radius
}

fn rect(at: BoxRect) -> Rect {
    Rect::new(at.x, at.y, at.x + at.width, at.y + at.height)
}

#[cfg(test)]
#[path = "clip_rounded_tests.rs"]
mod tests;
