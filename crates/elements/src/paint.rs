//! Paint: the laid-out boxes as a scene in device pixels, in stacking
//! order (`paint_order`), each under the clip of the boxes that cut it
//! off (`clip`), with every edge snapped to the device pixel grid as
//! Chromium snaps it.

use vapors_dom::NodeId;
use vapors_scene::{Rect, Scene, TransformClass};
use vapors_text::FontSystem;

use crate::layout::{BoxRect, LayoutBox};
use crate::paint_order::Stackings;
use crate::painted::Painted;
use crate::style::UsedMap;

/// What paint reads besides the boxes.
pub(crate) struct PaintInput<'a> {
    pub(crate) used: &'a UsedMap,
    /// Each slot's stacking, made once per update.
    pub(crate) stackings: &'a Stackings,
    /// `html` and `body`, whose backgrounds can become the canvas's.
    pub(crate) root: NodeId,
    pub(crate) body: NodeId,
    /// Which of them gave the canvas its colour; set by `paint`.
    pub(crate) propagated: Option<NodeId>,
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) cache: &'a mut crate::layout::TextCache,
    pub(crate) scale: f32,
    /// The clip of the box being painted; set by `paint`.
    pub(crate) clip: Option<crate::clip::BoxClip>,
    /// Each scroll container's offset and range, for its thumbs.
    pub(crate) scroll: &'a crate::scroll::Offsets,
    /// How opaque thumbs are drawn now, 0 (hidden) to 255 (on touch they
    /// show while scrolling and fade after).
    pub(crate) thumb_alpha: u8,
    /// The document's run-time shapes, which elements that paint shapes
    /// acquire from.
    pub(crate) shapes: &'a std::cell::RefCell<crate::document_shapes::DocumentShapes>,
    /// The placement painted from: whether each slot is a lane child in
    /// a frame of its own, which clips read, and each framed lane's
    /// container, which its children's frames are worked out from
    /// (`placement_frames.rs`, `frames_lane.rs`).
    pub(crate) placement: &'a crate::placement::Placement,
    /// The lanes, which lane frames read their shifts from.
    pub(crate) lanes: &'a crate::layout::SharedLanes,
    /// The class of the frame the box being painted is in, which decides
    /// whether its edges are snapped (`input.snapped`); set by `Repaint`
    /// before each box (`frames.rs` `FrameAt::class`).
    pub(crate) class: Option<TransformClass>,
    /// The boxes whose border box, items or clips hold a coordinate that
    /// is not finite in device pixels: painted, then refused by the
    /// update after paint (`paint_refusals.rs`), never shown.
    pub(crate) unresolvable: Vec<crate::paint_refusals::NotPlaced>,
    /// The boxes whose clip chain has more links than
    /// [`crate::MAX_CLIP_CHAIN`], with the count: refused by the update
    /// after paint (`paint_refusals.rs`).
    pub(crate) long_chains: Vec<(usize, usize)>,
}

impl<'a> PaintInput<'a> {
    /// The input for paint made from `from`: `page` is `html` and
    /// `body`, `text` the fonts and the text cache.
    pub(crate) fn new(
        (used, stackings): (&'a UsedMap, &'a Stackings),
        page: (NodeId, NodeId),
        (fonts, cache): (&'a mut FontSystem, &'a mut crate::layout::TextCache),
        (scroll, lanes, shapes): (
            &'a crate::scroll::Offsets,
            &'a crate::layout::SharedLanes,
            &'a std::cell::RefCell<crate::document_shapes::DocumentShapes>,
        ),
        from: &'a crate::kept_paint::PaintedFrom,
    ) -> Self {
        Self {
            used,
            stackings,
            root: page.0,
            body: page.1,
            propagated: None,
            fonts,
            cache,
            scale: from.scale,
            clip: None,
            scroll,
            thumb_alpha: from.thumb_alpha,
            shapes,
            placement: &from.placement,
            lanes,
            class: None,
            unresolvable: Vec::new(),
            long_chains: Vec::new(),
        }
    }

    /// `rect` in device pixels as the frame of the box being painted
    /// places it: snapped under a whole translation (`snapped`).
    ///
    /// # Panics
    /// No box's frame is set, or the frame is not a whole translation:
    /// boxes under other transforms are painted unsnapped from
    /// flat-scene F4a.
    pub(crate) fn snapped(&self, rect: BoxRect) -> Rect {
        snapped_in(self.class, rect, self.scale)
    }
}

/// `rect` in device pixels at `scale` as a frame of class `class` places
/// it ([`PaintInput::snapped`]).
///
/// # Panics
/// As [`PaintInput::snapped`].
pub(crate) fn snapped_in(class: Option<TransformClass>, rect: BoxRect, scale: f32) -> Rect {
    match class {
        Some(TransformClass::WholeTranslate) => snapped(rect, scale),
        Some(class) => panic!(
            "a box in a frame of class {class:?} is painted: only boxes under a whole \
             translation are painted before flat-scene F4a (docs/plans/flat-scene.md \
             section 8); reproduce with `cargo xtask test`"
        ),
        None => panic!(
            "a box's edges are placed with no frame class set: `Repaint` sets it before \
             each box (repaint.rs); reproduce with `cargo xtask test`"
        ),
    }
}

/// Paints `boxes` (box 0 the root) at the rects layout gave them, every
/// box afresh.
pub(crate) fn paint(boxes: &[LayoutBox], rects: &[BoxRect], input: &mut PaintInput<'_>) -> Painted {
    let changed = crate::changed_slots::ChangedSlots::all();
    crate::paint_partial::paint_partial(boxes, rects, input, None, &changed)
}

/// A paint with the canvas, before any box is painted; notes in
/// `input` which element gave the canvas its colour.
pub(crate) fn started(input: &mut PaintInput<'_>) -> Painted {
    let (canvas, propagated) = crate::canvas::canvas_background(input);
    input.propagated = propagated;
    let canvas_from = crate::canvas::canvas_from(propagated, input.root);
    Painted::empty(canvas, canvas_from, input.scale)
}

/// Sets the clip the next step paints under; a rounded clip takes the
/// radii of the box it names, one of `boxes` at `rects`.
pub(crate) fn clip_to(
    clip: Option<crate::clip::BoxClip>,
    (boxes, rects): (&[LayoutBox], &[BoxRect]),
    input: &mut PaintInput<'_>,
    scene: &mut Scene,
) {
    // Two forms of one clip, on purpose: hits are cut to it in CSS
    // pixels, unsnapped, as border boxes are hit unsnapped, so hit
    // testing follows layout geometry as Chromium's does; the scene
    // gets it snapped to device pixels, as every painted edge is.
    // They differ by under a device pixel at an edge.
    let radii_of = |slot| crate::clip::radii(boxes, rects, slot, input);
    let device = device_clip(clip, input.scale, &radii_of);
    input.clip = clip;
    scene.set_rounded_clip(device);
}

/// A clip in CSS pixels as the scene takes it: snapped to device pixels,
/// a rounded one with the radii `radii_of` gives the box it names,
/// scaled and fitted to the snapped rectangle. One answer for paint and
/// a kept part's shift (`kept_shift.rs`).
pub(crate) fn device_clip(
    clip: Option<crate::clip::BoxClip>,
    scale: f32,
    radii_of: &dyn Fn(usize) -> [f32; 4],
) -> Option<vapors_scene::Clip> {
    clip.map(|clip| {
        let rect = snapped(clip.rect, scale);
        match clip.rounded_by() {
            None => vapors_scene::Clip::square(rect),
            Some(slot) => {
                let radii = radii_of(slot).map(|r| r * scale);
                vapors_scene::Clip::rounded(rect, crate::corners::fitted(radii, rect))
            }
        }
    })
}

/// Box `index`'s own step: its own painting and its replaced content,
/// range, checkbox and text field marks.
pub(crate) fn paint_box(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
    out: &mut crate::painting::Painting,
) {
    let (layout_box, rect) = (&boxes[index], rects[index]);
    // a field's run: its marks under its text (`text_paint.rs`)
    crate::text_paint::paint(boxes, rects, index, input, out);
    crate::paint_own::paint_own(boxes, rects, index, input, out);
    crate::images::paint(layout_box, rect, input, out);
    crate::svg_paint::paint(layout_box, rect, input, out);
    crate::surfaces::paint(layout_box, rect, input, out);
    crate::range_paint::paint(layout_box, rect, input, out);
    crate::checkbox_paint::paint(layout_box, rect, input, out);
    crate::switch::paint(layout_box, rect, input, out);
    // an empty field's placeholder, on its own box (`placeholder_paint.rs`)
    crate::placeholder_paint::paint(boxes, rects, index, input, out);
}

/// `rect` in device pixels, each edge rounded to the nearest pixel, so
/// two boxes that share an edge in CSS pixels share it on screen.
pub(crate) fn snapped(rect: BoxRect, scale: f32) -> Rect {
    Rect::new(
        half_up(rect.x * scale),
        half_up(rect.y * scale),
        half_up((rect.x + rect.width) * scale),
        half_up((rect.y + rect.height) * scale),
    )
}

/// Whether every edge of `rect` is finite: a box placed in device
/// pixels, rather than one paint refuses (`PaintInput::unresolvable`).
pub(crate) fn finite(rect: Rect) -> bool {
    [rect.left, rect.top, rect.right, rect.bottom]
        .iter()
        .all(|edge| edge.is_finite())
}

/// `v` rounded to the nearest whole number, a half up, as Chromium's
/// `LayoutUnit::Round` rounds: unlike rounding half away from zero,
/// `half_up(v - n)` is `half_up(v) - n` for a whole `n`, so an edge
/// snapped in its scroll frame and shifted by a whole offset is where
/// it snaps on the page (`docs/plans/iu4.md` section 3).
/// Not `(v + 0.5).floor()`, whose sum rounds 0.49999997 up to 1.
// Exact: `v - v.round()` is exact for a float and its nearest whole.
#[allow(clippy::float_cmp)]
fn half_up(v: f32) -> f32 {
    let nearest = v.round();
    if v - nearest == 0.5 {
        nearest + 1.0
    } else {
        nearest
    }
}

#[cfg(test)]
#[path = "paint_tests.rs"]
mod tests;
