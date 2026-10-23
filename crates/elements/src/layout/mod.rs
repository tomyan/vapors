//! Layout: the box tree laid out by Taffy in CSS pixels, unrounded, and
//! each box's border box in document coordinates.

mod boxes;
mod calcs;
mod inline;
mod keep;
mod measure;
mod text_cache;
mod tree;

use taffy::{AvailableSpace, NodeId, Size, compute_root_layout};
use vapors_text::FontSystem;

pub(crate) use boxes::{Content, LayoutBox, build};
pub(crate) use calcs::Calcs;
pub(crate) use inline::{InlineRun, InlineSpan};
pub(crate) use keep::KeptBoxes;
pub(crate) use text_cache::TextCache;
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

/// Lays `boxes` out in a viewport of `size` CSS pixels, which is the
/// initial containing block: the root's percentages resolve against it,
/// and an `auto` height is still its content's. A box unchanged since
/// the update `kept` holds takes its cache and layout first. Returns
/// the boxes with each one's border box, in box order, and how many
/// times Taffy laid a box out rather than reading its cache.
///
/// # Errors
/// A `calc()` that did not resolve (see `Calcs`).
pub(crate) fn lay_out(
    mut boxes: Vec<LayoutBox>,
    texts: (&mut FontSystem, &mut TextCache),
    kept: (&KeptBoxes, &dyn Fn(vapors_dom::NodeId) -> bool),
    scale: f32,
    size: (f32, f32),
    calcs: &Calcs,
) -> Result<(Vec<LayoutBox>, Vec<BoxRect>, usize), String> {
    let (fonts, cache) = texts;
    kept.0.lend(&mut boxes, scale, kept.1);
    let mut tree = BoxTree {
        boxes,
        fonts,
        cache,
        scale,
        calcs,
        laid_out: 0,
    };
    let available = Size {
        width: AvailableSpace::Definite(size.0),
        height: AvailableSpace::Definite(size.1),
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
    let containing = containing_blocks(&tree.boxes);
    place(&tree.boxes, &containing, 0, (0.0, 0.0), &mut rects);
    match calcs.failure() {
        Some(failure) => Err(failure),
        None => Ok((tree.boxes, rects, tree.laid_out)),
    }
}

/// Each out-of-flow box's containing block, by box index.
pub(crate) fn containing_blocks(boxes: &[LayoutBox]) -> Vec<Option<usize>> {
    let mut containing = vec![None; boxes.len()];
    for (index, layout_box) in boxes.iter().enumerate() {
        for &hoisted in &layout_box.hoisted {
            containing[hoisted] = Some(index);
        }
    }
    containing
}

/// Places `index` at `origin` (its parent's corner, or its containing
/// block's when it is out of flow, which is placed first: an ancestor)
/// and its children after it.
fn place(
    boxes: &[LayoutBox],
    containing: &[Option<usize>],
    index: usize,
    origin: (f32, f32),
    rects: &mut [BoxRect],
) {
    let layout = &boxes[index].layout;
    let origin = containing[index].map_or(origin, |block| (rects[block].x, rects[block].y));
    let (x, y) = (origin.0 + layout.location.x, origin.1 + layout.location.y);
    rects[index] = BoxRect {
        x,
        y,
        width: layout.size.width,
        height: layout.size.height,
    };
    for &child in &boxes[index].children {
        place(boxes, containing, child, (x, y), rects);
    }
}
