//! Layout: the box tree laid out by Taffy in CSS pixels, unrounded, and
//! each box's border box in document coordinates.

mod asked;
mod boxes;
mod calcs;
mod child_edits;
mod child_edits_check;
mod field_box;
mod inline;
mod isolated;
mod keep;
mod lane;
mod lane_block;
mod lane_block_answer;
mod lane_block_child;
mod lane_block_child_place;
mod lane_block_container;
mod lane_block_item;
mod lane_block_sizes;
mod lane_block_summary;
mod lane_bounds;
#[cfg(test)]
mod lane_compare;
// The flex lane's fold (docs/plans/splice-layout.md, PR 2); layout
// reads it from PR 3f, so until then only the tests do.
mod lane_fill;
// The flex lane's mirror (docs/plans/splice-layout.md, PR 3f-1); layout
// routes columns to it in PR 3f-2, so until then only the tests do.
#[allow(dead_code)]
mod lane_flex;
#[allow(dead_code)]
mod lane_flex_answer;
#[allow(dead_code)]
mod lane_flex_base;
#[allow(dead_code)]
mod lane_flex_child;
#[allow(dead_code)]
mod lane_flex_column;
#[allow(dead_code)]
mod lane_flex_contribution;
#[allow(dead_code)]
mod lane_flex_item;
#[allow(dead_code)]
mod lane_flex_leaf;
#[allow(dead_code)]
mod lane_flex_place;
#[allow(dead_code)]
mod lane_flex_sizes;
#[allow(dead_code)]
mod lane_flex_summary;
mod lane_layout;
#[cfg(test)]
mod lane_page;
mod lane_share;
mod lane_tree;
mod layout_box;
#[cfg(debug_assertions)]
pub(crate) mod layout_check;
mod marks;
mod measure;
mod owners;
mod rebuild;
#[cfg(debug_assertions)]
mod rebuild_check;
mod rebuilt;
mod replaced;
mod slots;
pub(crate) use boxes::Source;
mod text_cache;
mod text_kept;
mod text_key;
mod tree;
mod work;
mod written;

use taffy::{AvailableSpace, NodeId, Size, compute_root_layout};
use vapors_text::FontSystem;

pub(crate) use boxes::{Content, LayoutBox};
pub(crate) use calcs::Calcs;
pub(crate) use inline::{InlineRun, InlineSpan};
pub(crate) use keep::KeptBoxes;
pub(crate) use lane_layout::LaneCounts;
pub(crate) use rebuilt::Rebuilt;
pub(crate) use slots::{Key, Placed, Slots};
pub(crate) use text_cache::TextCache;
use tree::BoxTree;
pub(crate) use work::{LayoutWork, Settling};
pub(crate) use written::Written;

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
/// Each of `settling.changed`, a rebuilt owner whose subtree changed,
/// is first settled (`isolated.rs`): laid out in isolation when that
/// answers as the box it replaced did, else its ancestors are laid out
/// again. A block container is laid out by its lane where it qualifies
/// (`lane_layout.rs`), its children `settling.built` this update laid
/// out again.
///
/// # Errors
/// A `calc()` that did not resolve (see `Calcs`), with what Taffy did
/// (boxed: the pair is wider than clippy's `Err` limit).
pub(crate) fn lay_out(
    mut placed: Placed,
    texts: (&mut FontSystem, &mut TextCache),
    scale: f32,
    size: (f32, f32),
    calcs: &Calcs,
    settling: Settling,
) -> Result<(Placed, LayoutWork), Box<(String, LayoutWork)>> {
    let (fonts, cache) = texts;
    let mut tree = BoxTree {
        boxes: placed.boxes,
        fonts,
        cache,
        scale,
        calcs,
        written: Written::default(),
        calls: 0,
        disowned: 0,
        lanes: &mut placed.lanes,
        lanes_enabled: settling.lanes,
        stale: settling.built.into_iter().collect(),
        lane_counts: LaneCounts::default(),
    };
    let changed = settling.changed;
    let available = Size {
        width: AvailableSpace::Definite(size.0),
        height: AvailableSpace::Definite(size.1),
    };
    let settled = tree.settle(changed);
    // With html `display: none` there is no box, so no root to lay out.
    if !tree.boxes.is_empty() {
        compute_root_layout(&mut tree, NodeId::from(0usize), available);
    }
    let work = LayoutWork {
        laid_out: tree.written.laid_out.len(),
        calls: tree.calls,
        disowned: tree.disowned,
        settled,
        lanes: tree.lane_counts,
        written: std::mem::take(&mut tree.written),
    };
    let boxes = tree.boxes;
    match calcs.failure() {
        Some(failure) => Err(Box::new((failure, work))),
        None => Ok((
            Placed {
                boxes,
                slots: placed.slots,
                lanes: placed.lanes,
            },
            work,
        )),
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
/// its scroll offset (`offsets`, by box index, in CSS pixels); and each
/// box's padding, read in the same pass.
pub(crate) fn placed(
    boxes: &[LayoutBox],
    offsets: &[[f32; 2]],
) -> (Vec<BoxRect>, Vec<taffy::Rect<f32>>) {
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
            let layout = &boxes[index].layout;
            let (width, height) = (layout.size.width, layout.size.height);
            (
                BoxRect {
                    x,
                    y,
                    width,
                    height,
                },
                layout.padding,
            )
        })
        .unzip()
}

#[cfg(test)]
#[path = "lay_out_tests.rs"]
mod tests;
