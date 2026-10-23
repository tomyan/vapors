//! Paint: the laid-out boxes as a scene in device pixels, in tree order
//! (each element's background, then its children), with every edge
//! snapped to the device pixel grid as Chromium snaps it.

use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_scene::{Color, Item, Quad, Rect, Scene, TextId};
use vapors_text::{FontSystem, Layout};

use crate::layout::{BoxRect, Content, LayoutBox, text_layout};
use crate::style::Used;

/// The result of an update: the canvas colour, the scene, the text runs
/// it names, and each element's border box.
pub struct Painted {
    pub(crate) canvas: Color,
    pub(crate) scene: Scene,
    pub(crate) texts: Vec<Layout>,
    pub(crate) rects: HashMap<NodeId, BoxRect>,
}

/// What paint reads besides the boxes.
pub(crate) struct PaintInput<'a> {
    pub(crate) used: &'a HashMap<NodeId, Used>,
    /// `html` and `body`, whose backgrounds can become the canvas's.
    pub(crate) root: NodeId,
    pub(crate) body: NodeId,
    /// Which of them gave the canvas its colour; set by `paint`.
    pub(crate) propagated: Option<NodeId>,
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) scale: f32,
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
    };
    for (index, layout_box) in boxes.iter().enumerate() {
        if let Content::Element(node) = layout_box.content {
            painted.rects.insert(node, rects[index]);
        }
    }
    paint_box(boxes, rects, 0, input, &mut painted);
    painted
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

fn paint_box(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    index: usize,
    input: &mut PaintInput<'_>,
    out: &mut Painted,
) {
    let rect = rects[index];
    match &boxes[index].content {
        Content::Element(node) => {
            let background = input.used[node].background;
            if background.a > 0.0 && input.propagated != Some(*node) {
                let rect = snapped(rect, input.scale);
                out.scene.push(Item::Quad(Quad { rect, background }));
            }
        }
        Content::Text { text, style } => {
            let layout = text_layout(input.fonts, input.scale, text, style, Some(rect.width));
            let id = TextId(u32::try_from(out.texts.len()).expect("fewer than 2^32 text runs"));
            out.texts.push(layout);
            let origin = [rect.x * input.scale, rect.y * input.scale];
            out.scene.push(Item::Text { text: id, origin });
        }
    }
    for &child in &boxes[index].children {
        paint_box(boxes, rects, child, input, out);
    }
}

/// `rect` in device pixels, each edge rounded to the nearest pixel, so
/// two boxes that share an edge in CSS pixels share it on screen.
fn snapped(rect: BoxRect, scale: f32) -> Rect {
    Rect::new(
        (rect.x * scale).round(),
        (rect.y * scale).round(),
        ((rect.x + rect.width) * scale).round(),
        ((rect.y + rect.height) * scale).round(),
    )
}

impl Painted {
    /// The colour under everything: what the window is cleared to.
    #[must_use]
    pub fn canvas(&self) -> Color {
        self.canvas
    }

    /// The display list.
    #[must_use]
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// The text run a scene item names.
    ///
    /// # Panics
    /// When `id` is not from this update's scene.
    #[must_use]
    pub fn text(&self, id: TextId) -> &Layout {
        &self.texts[id.0 as usize]
    }

    /// The element's border box in CSS pixels, or `None` when it has no
    /// box (`display: none`, or outside the document).
    #[must_use]
    pub fn border_box(&self, node: NodeId) -> Option<BoxRect> {
        self.rects.get(&node).copied()
    }
}
