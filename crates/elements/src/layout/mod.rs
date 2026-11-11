//! Layout: the box tree laid out by Taffy in CSS pixels, unrounded, and
//! each box's border box in document coordinates.

mod boxes;
mod calcs;
mod field_box;
mod inline;
mod keep;
mod layout_box;
mod marks;
mod measure;
mod rebuild;
mod replaced;
mod slots;
pub(crate) use boxes::Source;
mod text_cache;
mod text_key;
mod tree;

use taffy::{AvailableSpace, NodeId, Size, compute_root_layout};
use vapors_text::FontSystem;

pub(crate) use boxes::{Content, LayoutBox};
pub(crate) use calcs::Calcs;
pub(crate) use inline::{InlineRun, InlineSpan};
pub(crate) use keep::KeptBoxes;
pub(crate) use slots::Placed;
pub(crate) use text_cache::TextCache;
use tree::BoxTree;

/// A box's border box in document coordinates, CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// Lays the `placed` boxes out in a viewport of `size` CSS pixels, which is the
/// initial containing block: the root's percentages resolve against it,
/// and an `auto` height is still its content's. A box lent its cache
/// (`KeptBoxes::rebuild`) is not laid out again. Returns
/// them and what Taffy did (`LayoutWork`); `placed` gives their
/// border boxes. An empty tree (html `display: none`) lays out nothing.
///
/// # Errors
/// A `calc()` that did not resolve (see `Calcs`), with what Taffy did.
pub(crate) fn lay_out(
    placed: Placed,
    texts: (&mut FontSystem, &mut TextCache),
    scale: f32,
    size: (f32, f32),
    calcs: &Calcs,
) -> Result<(Placed, LayoutWork), (String, LayoutWork)> {
    let (fonts, cache) = texts;
    let mut tree = BoxTree {
        boxes: placed.boxes,
        fonts,
        cache,
        scale,
        calcs,
        laid_out: 0,
        calls: 0,
    };
    let available = Size {
        width: AvailableSpace::Definite(size.0),
        height: AvailableSpace::Definite(size.1),
    };
    // With html `display: none` there is no box, so no root to lay out.
    if !tree.boxes.is_empty() {
        compute_root_layout(&mut tree, NodeId::from(0usize), available);
    }
    let work = LayoutWork {
        laid_out: tree.laid_out,
        calls: tree.calls,
    };
    match calcs.failure() {
        Some(failure) => Err((failure, work)),
        None => Ok((
            Placed {
                boxes: tree.boxes,
                slots: placed.slots,
            },
            work,
        )),
    }
}

/// What one layout did: how many times Taffy laid a box out rather than
/// reading its cache, and how many times it asked a box for its layout.
pub(crate) struct LayoutWork {
    pub(crate) laid_out: usize,
    pub(crate) calls: usize,
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

#[cfg(test)]
#[path = "lay_out_tests.rs"]
mod tests;
