//! Layout: the box tree laid out by Taffy in CSS pixels, unrounded, and
//! each box's border box in document coordinates.

mod boxes;
mod measure;
mod tree;

use taffy::{AvailableSpace, NodeId, Size, compute_root_layout};
use vapors_text::FontSystem;

pub(crate) use boxes::{Content, LayoutBox, build};
pub(crate) use measure::text_layout;
use tree::BoxTree;

/// A box's border box in document coordinates, CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// Lays `boxes` out in a viewport `width` CSS pixels wide, and returns
/// them with each box's border box, in box order.
pub(crate) fn lay_out(
    boxes: Vec<LayoutBox>,
    fonts: &mut FontSystem,
    scale: f32,
    width: f32,
) -> (Vec<LayoutBox>, Vec<BoxRect>) {
    let mut tree = BoxTree {
        boxes,
        fonts,
        scale,
    };
    let available = Size {
        width: AvailableSpace::Definite(width),
        height: AvailableSpace::MaxContent,
    };
    compute_root_layout(&mut tree, NodeId::from(0usize), available);
    let mut rects = vec![
        BoxRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0
        };
        tree.boxes.len()
    ];
    place(&tree.boxes, 0, (0.0, 0.0), &mut rects);
    (tree.boxes, rects)
}

fn place(boxes: &[LayoutBox], index: usize, origin: (f32, f32), rects: &mut [BoxRect]) {
    let layout = &boxes[index].layout;
    let (x, y) = (origin.0 + layout.location.x, origin.1 + layout.location.y);
    rects[index] = BoxRect {
        x,
        y,
        width: layout.size.width,
        height: layout.size.height,
    };
    for &child in &boxes[index].children {
        place(boxes, child, (x, y), rects);
    }
}
