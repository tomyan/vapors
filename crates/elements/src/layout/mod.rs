//! Layout: the box tree laid out by Taffy in CSS pixels, unrounded, and
//! each box's border box in document coordinates.

mod boxes;
mod calcs;
mod inline;
mod keep;
mod measure;
mod replaced;
pub(crate) use boxes::{NumberFocus, Source};
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
/// the boxes and how many times Taffy laid a box out rather than
/// reading its cache; `placed` gives their border boxes.
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
) -> Result<(Vec<LayoutBox>, usize), String> {
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
    match calcs.failure() {
        Some(failure) => Err(failure),
        None => Ok((tree.boxes, tree.laid_out)),
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

/// Each box's border box in document coordinates: its place in its
/// containing block's content, whose origin is that box's corner less
/// its scroll offset (`offsets`, by box index, in CSS pixels).
pub(crate) fn placed(boxes: &[LayoutBox], offsets: &[[f32; 2]]) -> Vec<BoxRect> {
    let corner = |index: usize, origin: (f32, f32)| {
        let location = boxes[index].layout.location;
        (origin.0 + location.x, origin.1 + location.y)
    };
    let origins = crate::chain::by_containing_block(boxes, (0.0, 0.0), &|index, origin| {
        let (x, y) = corner(index, origin);
        (x - offsets[index][0], y - offsets[index][1])
    });
    (0..boxes.len())
        .map(|index| {
            let (x, y) = corner(index, origins[index]);
            let size = boxes[index].layout.size;
            BoxRect {
                x,
                y,
                width: size.width,
                height: size.height,
            }
        })
        .collect()
}
