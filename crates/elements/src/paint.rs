//! Paint: the laid-out boxes as a scene in device pixels, in stacking
//! order (`paint_order`), each under the clip of the boxes that cut it
//! off (`clip`), with every edge snapped to the device pixel grid as
//! Chromium snaps it.

use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Quad, Rect, Scene, Shadow, TextId};
use vapors_text::FontSystem;

use crate::corners;
use crate::layout::{BoxRect, Content, InlineRun, LayoutBox};
use crate::paint_order::{Stacking, paint_order};
use crate::painted::Painted;
use crate::style::{BoxShadow, Used};

/// What paint reads besides the boxes.
pub(crate) struct PaintInput<'a> {
    pub(crate) used: &'a HashMap<NodeId, Used>,
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
}

/// Paints `boxes` (box 0 the root) at the rects layout gave them.
pub(crate) fn paint(boxes: &[LayoutBox], rects: &[BoxRect], input: &mut PaintInput<'_>) -> Painted {
    let (canvas, propagated) = canvas_background(input);
    input.propagated = propagated;
    let mut painted = Painted {
        canvas,
        scene: Scene::new(),
        texts: Vec::new(),
        rects: HashMap::new(),
        hits: Vec::new(),
        cursors: HashMap::new(),
    };
    for (index, layout_box) in boxes.iter().enumerate() {
        if let Content::Element(node) = layout_box.content {
            painted.rects.insert(node, rects[index]);
        }
    }
    let order = paint_order(boxes, &|index| stacking_of(boxes, index, input));
    let clips = crate::clip::clips(boxes, rects, &|index| clipping(boxes, index, input));
    for index in order {
        // Two forms of one clip, on purpose: hits are cut to it in CSS
        // pixels, unsnapped, as border boxes are hit unsnapped, so hit
        // testing follows layout geometry as Chromium's does; the scene
        // gets it snapped to device pixels, as every painted edge is.
        // They differ by under a device pixel at an edge.
        input.clip = clips[index];
        let device = clips[index].map(|clip| snapped(clip, input.scale));
        painted.scene.set_clip(device);
        paint_own(boxes, rects, index, input, &mut painted);
    }
    painted
}

/// Whether box `index` cuts off its content. The root never does: its
/// `overflow` is the viewport's. Nor does the body unless the root's
/// `overflow` is not `visible`; otherwise the body's is the viewport's
/// (CSS Overflow 3 section 3.3).
fn clipping(boxes: &[LayoutBox], index: usize, input: &PaintInput<'_>) -> bool {
    let Content::Element(node) = boxes[index].content else {
        return false;
    };
    let clips = |node: NodeId| input.used.get(&node).is_some_and(Used::clips);
    match node {
        node if node == input.root => false,
        node if node == input.body => clips(node) && clips(input.root),
        node => clips(node),
    }
}

fn stacking_of(boxes: &[LayoutBox], index: usize, input: &PaintInput<'_>) -> Stacking {
    let Content::Element(node) = boxes[index].content else {
        return Stacking::default();
    };
    let used = &input.used[&node];
    Stacking {
        // a z-index is kept only where it applies: on a static box, a
        // flex or grid item, which then stacks as if positioned
        positioned: used.layout.position != taffy::Position::Static || used.z_index.is_some(),
        z: used.z_index,
        fixed: used.layout.position == taffy::Position::Fixed,
    }
}

/// The canvas's colour, as CSS Backgrounds 3 section 2.11.2 gives it:
/// the root's background, else the body's when the root's is
/// transparent, else white; and the element whose background it took,
/// which then paints none of its own.
fn canvas_background(input: &PaintInput<'_>) -> (Color, Option<NodeId>) {
    for node in [input.root, input.body] {
        match input.used.get(&node) {
            Some(used) if used.background.a > 0.0 => return (used.background, Some(node)),
            _ => {}
        }
    }
    (Color::from_rgba8(255, 255, 255, 255), None)
}

/// One box's own painting: an element's shadows, background and
/// border, or a run's text; its children are painted in their turn.
fn paint_own(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
    out: &mut Painted,
) {
    match &boxes[index].content {
        Content::Element(node) => paint_element(*node, rects[index], input, out),
        Content::Text(run) => paint_text(run, rects[index], input, out),
    }
}

/// The element's shadows, then its background and border, unless it is
/// hidden; a background that became the canvas's is not painted again.
/// Also its place among the elements the pointer can hit.
fn paint_element(node: NodeId, rect: BoxRect, input: &PaintInput<'_>, out: &mut Painted) {
    let used = &input.used[&node];
    if used.visible && used.takes_pointer {
        out.hits.push((node, crate::clip::within(input.clip, rect)));
        out.cursors.insert(node, used.cursor);
    }
    if !used.visible {
        return;
    }
    let border_box = snapped(rect, input.scale);
    let decoration = &used.decoration;
    let radii = corners::fitted(decoration.radii.map(|r| r * input.scale), border_box);
    for shadow in decoration.shadows.iter().rev() {
        out.scene.push(Item::Shadow(shadow_item(
            shadow,
            border_box,
            radii,
            input.scale,
        )));
    }
    let background = if input.propagated == Some(node) {
        Color::default()
    } else {
        used.background
    };
    let border = decoration.border.map(|w| w * input.scale);
    if background.a > 0.0 || border.iter().any(|w| *w > 0.0) {
        let quad = Quad {
            rect: border_box,
            background,
            border,
            border_colors: decoration.border_colors,
            radii,
        };
        out.scene.push(Item::Quad(quad));
    }
}

/// A CSS outer shadow as the scene draws it: the border box moved by
/// the offset and grown by the spread, corners grown with it.
fn shadow_item(shadow: &BoxShadow, caster: Rect, radii: [f32; 4], scale: f32) -> Shadow {
    let [dx, dy] = shadow.offset.map(|v| v * scale);
    let spread = shadow.spread * scale;
    Shadow {
        rect: Rect::new(
            caster.left + dx - spread,
            caster.top + dy - spread,
            caster.right + dx + spread,
            caster.bottom + dy + spread,
        ),
        radii: radii.map(|r| corners::spread(r, spread)),
        blur: shadow.blur * scale,
        color: shadow.color,
        caster,
        caster_radii: radii,
    }
}

fn paint_text(run: &InlineRun, rect: BoxRect, input: &mut PaintInput<'_>, out: &mut Painted) {
    let layout = input
        .cache
        .layout(input.fonts, input.scale, run, Some(rect.width));
    crate::paint_inline::paint_spans(run, &layout, rect, input, out);
    let id = TextId(u32::try_from(out.texts.len()).expect("fewer than 2^32 text runs"));
    out.texts.push(layout);
    let origin = [rect.x * input.scale, rect.y * input.scale];
    out.scene.push(Item::Text { text: id, origin });
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
