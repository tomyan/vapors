//! Paint: the laid-out boxes as a scene in device pixels, in stacking
//! order (`paint_order`), each under the clip of the boxes that cut it
//! off (`clip`), with every edge snapped to the device pixel grid as
//! Chromium snaps it.

use crate::node_map::NodeMap;

use vapors_dom::NodeId;
use vapors_scene::{Rect, Scene};
use vapors_text::FontSystem;

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint_order::{Step, paint_steps_with_outlines, stacking_of};
use crate::painted::Painted;
use crate::style::Used;

/// What paint reads besides the boxes.
pub(crate) struct PaintInput<'a> {
    pub(crate) used: &'a NodeMap<Used>,
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
        used: &'a NodeMap<Used>,
        page: (NodeId, NodeId),
        (fonts, cache): (&'a mut FontSystem, &'a mut crate::layout::TextCache),
        scroll: &'a crate::scroll::Offsets,
        from: &crate::kept_paint::PaintedFrom,
    ) -> Self {
        Self {
            used,
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

/// Paints `boxes` (box 0 the root) at the rects layout gave them.
pub(crate) fn paint(boxes: &[LayoutBox], rects: &[BoxRect], input: &mut PaintInput<'_>) -> Painted {
    let mut painted = started(boxes, rects, input);
    let (steps, clips) = order(boxes, rects, input);
    for step in steps {
        let index = step.index();
        clip_to(clips[index], input, &mut painted.scene);
        match step {
            Step::Box(_) => {
                painted.boxes_drawn += 1;
                paint_box(boxes, rects, index, input, &mut painted);
            }
            Step::Thumb(_) => crate::thumb::paint(&boxes[index], rects[index], input, &mut painted),
            Step::Outline(_) => {
                crate::paint_outline::paint_outline(
                    &boxes[index],
                    rects[index],
                    input,
                    &mut painted,
                );
            }
        }
    }
    painted.items_built = painted.scene.items().len();
    painted
}

/// A paint with the canvas and every element's border and padding box,
/// before any box is painted; notes in `input` which element gave the
/// canvas its colour.
pub(crate) fn started(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &mut PaintInput<'_>,
) -> Painted {
    let (canvas, propagated) = crate::canvas::canvas_background(input);
    input.propagated = propagated;
    let canvas_from = crate::canvas::canvas_from(propagated, input.root);
    let mut painted = Painted::empty(canvas, canvas_from, input.scale);
    for (index, layout_box) in boxes.iter().enumerate() {
        if let Content::Element(node) = layout_box.content {
            painted.rects.insert(node, rects[index]);
            let padding = crate::clip::padding_box(&boxes[index], rects[index]);
            painted.paddings.insert(node, [padding.x, padding.y]);
        }
    }
    painted
}

/// The steps of painting in order, and each box's clip.
pub(crate) fn order(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    input: &PaintInput<'_>,
) -> (Vec<Step>, Vec<Option<BoxRect>>) {
    let steps = paint_steps_with_outlines(boxes, &|index| stacking_of(boxes, index, input.used));
    let clips = crate::clip::clips(boxes, rects, &|index| {
        crate::clip::clipping(boxes, index, input)
    });
    (steps, clips)
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
    out: &mut Painted,
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
        (rect.x * scale).round(),
        (rect.y * scale).round(),
        ((rect.x + rect.width) * scale).round(),
        ((rect.y + rect.height) * scale).round(),
    )
}
