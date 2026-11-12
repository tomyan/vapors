//! Paint: the laid-out boxes as a scene in device pixels, in stacking
//! order (`paint_order`), each under the clip of the boxes that cut it
//! off (`clip`), with every edge snapped to the device pixel grid as
//! Chromium snaps it.

use crate::node_map::NodeMap;

use vapors_dom::NodeId;
use vapors_scene::{Rect, Scene};
use vapors_text::FontSystem;

use crate::layout::{BoxRect, LayoutBox};
use crate::paint_order::Stackings;
use crate::painted::Painted;
use crate::style::Used;

/// What paint reads besides the boxes.
pub(crate) struct PaintInput<'a> {
    pub(crate) used: &'a NodeMap<Used>,
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
    pub(crate) clip: Option<BoxRect>,
    /// Each scroll container's offset and range, for its thumbs.
    pub(crate) scroll: &'a crate::scroll::Offsets,
    /// Whether thumbs are drawn now (on touch, only while scrolling).
    pub(crate) thumbs_shown: bool,
}

impl<'a> PaintInput<'a> {
    /// The input for paint made from `from`: `page` is `html` and
    /// `body`, `text` the fonts and the text cache.
    pub(crate) fn new(
        (used, stackings): (&'a NodeMap<Used>, &'a Stackings),
        page: (NodeId, NodeId),
        (fonts, cache): (&'a mut FontSystem, &'a mut crate::layout::TextCache),
        scroll: &'a crate::scroll::Offsets,
        from: &crate::kept_paint::PaintedFrom,
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
            thumbs_shown: from.thumbs_shown,
        }
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

/// Sets the clip the next step paints under.
pub(crate) fn clip_to(clip: Option<BoxRect>, input: &mut PaintInput<'_>, scene: &mut Scene) {
    // Two forms of one clip, on purpose: hits are cut to it in CSS
    // pixels, unsnapped, as border boxes are hit unsnapped, so hit
    // testing follows layout geometry as Chromium's does; the scene
    // gets it snapped to device pixels, as every painted edge is.
    // They differ by under a device pixel at an edge.
    input.clip = clip;
    scene.set_clip(clip.map(|clip| snapped(clip, input.scale)));
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
    crate::paint_own::paint_own(boxes, rects, index, input, out);
    crate::images::paint(layout_box, rect, input, out);
    crate::svg_paint::paint(layout_box, rect, input, out);
    crate::surfaces::paint(layout_box, rect, input, out);
    crate::range_paint::paint(layout_box, rect, input, out);
    crate::checkbox_paint::paint(layout_box, rect, input, out);
    crate::text_paint::paint(boxes, rects, index, input, out);
}

/// `rect` in device pixels, each edge rounded to the nearest pixel, so
/// two boxes that share an edge in CSS pixels share it on screen.
pub(crate) fn snapped(rect: BoxRect, scale: f32) -> Rect {
    Rect::new(
        whole(rect.x * scale),
        whole(rect.y * scale),
        whole((rect.x + rect.width) * scale),
        whole((rect.y + rect.height) * scale),
    )
}

/// `x` rounded to the nearest whole pixel, never -0.0: runs compare
/// -0.0 equal to 0.0, so a kept batch's bytes must not hold either.
fn whole(x: f32) -> f32 {
    x.round() + 0.0
}
