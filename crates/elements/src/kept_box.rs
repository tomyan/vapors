//! One box's kept paint (IU3 3c-2, `docs/plans/iu3.md`): what its own
//! step and its outline put into the paint, its items naming ids in the
//! paint's slabs (`slab.rs`), and what it was made from besides its box. A box kept from the last
//! update (`LayoutBox::kept`) whose values below are unchanged has its
//! kept paint copied (`paint_partial.rs`); any other is painted again.

use vapors_dom::NodeId;
use vapors_scene::{Item, Scene};

use crate::layout::{BoxRect, LayoutBox};
use crate::painted::Painted;
use crate::style::Cursor;

/// What a box's paint reads besides its box and style. Not here, as
/// they change only with a rebuilt box: border widths, which come only
/// from style (CSS has no % border); and whether its background became
/// the canvas's, which reads only html's and body's style, whose
/// restyle rebuilds the tree or body; and a text field's text origin,
/// which follows from its rect, padding, border and style, while a
/// value or style change rebuilds the field. The debug check
/// (`kept_paint_check.rs`) guards all three.
#[derive(PartialEq)]
pub(crate) struct BoxFrom {
    /// Its border box, scroll offsets applied.
    pub(crate) rect: BoxRect,
    /// The clip of the boxes that cut it off.
    pub(crate) clip: Option<BoxRect>,
    /// Its padding widths, for its padding and content box (% padding
    /// follows the containing block's width).
    pub(crate) padding: taffy::Rect<f32>,
}

/// One box's kept paint.
pub(crate) struct KeptBox {
    pub(crate) from: BoxFrom,
    /// Its own step's paint.
    pub(crate) own: Part,
    /// Its outline's items.
    pub(crate) outline: Vec<Item>,
}

/// What one step put into a paint; its items name ids its paint's
/// slabs hold while the part is kept.
#[derive(Default)]
pub(crate) struct Part {
    scene: Scene,
    hits: Vec<(NodeId, BoxRect)>,
    cursors: Vec<(NodeId, Cursor)>,
    visible: Vec<NodeId>,
    rects: Vec<(NodeId, BoxRect)>,
    ranges: Vec<(NodeId, crate::range_paint::Track)>,
    text_origins: Vec<(NodeId, [f32; 2])>,
}

impl BoxFrom {
    /// What box `index`'s paint reads now, under `clip`.
    pub(crate) fn of(
        boxes: &[LayoutBox],
        rects: &[BoxRect],
        clip: Option<BoxRect>,
        index: usize,
    ) -> Self {
        Self {
            rect: rects[index],
            clip,
            padding: boxes[index].layout.padding,
        }
    }
}

impl KeptBox {
    /// How many scene items this box's paint holds.
    pub(crate) fn items(&self) -> usize {
        self.own.scene.items().len() + self.outline.len()
    }

    /// Each scene item this box's paint holds: its own step's, then its
    /// outline's.
    pub(crate) fn all_items(&self) -> impl Iterator<Item = &Item> {
        self.own.scene.items().iter().chain(&self.outline)
    }

    /// Frees the slab ids this box's paint names, as it is dropped.
    pub(crate) fn free(&self, slabs: &mut crate::slab::Slabs) {
        slabs.free(self.own.scene.items());
        slabs.free(&self.outline);
    }
}

impl Part {
    /// What `scratch` holds but its slabs, leaving it empty.
    pub(crate) fn take(scratch: &mut Painted) -> Self {
        use std::mem::take;
        Self {
            scene: take(&mut scratch.scene),
            hits: take(&mut scratch.hits),
            cursors: scratch.cursors.drain().collect(),
            visible: scratch.visible.drain().collect(),
            rects: scratch.rects.drain().collect(),
            ranges: scratch.ranges.drain().collect(),
            text_origins: scratch.text_origins.drain().collect(),
        }
    }

    /// The items, each under its clip, then the rest, into `out`; the
    /// ids they name are `out`'s already.
    pub(crate) fn copy_into(&self, out: &mut Painted) {
        for (item, clip) in self.scene.items().iter().zip(self.scene.clips()) {
            out.scene.set_clip(*clip);
            out.scene.push(*item);
        }
        out.hits.extend_from_slice(&self.hits);
        out.cursors.extend(self.cursors.iter().copied());
        out.visible.extend(self.visible.iter().copied());
        out.rects.extend(self.rects.iter().copied());
        out.ranges.extend(self.ranges.iter().copied());
        out.text_origins.extend(self.text_origins.iter().copied());
    }
}
