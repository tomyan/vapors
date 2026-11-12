//! One box's kept paint (IU3 3c-2, `docs/plans/iu3.md`): what its own
//! step, its outline and its thumbs put into the paint, its items
//! naming ids in the paint's slabs (`slab.rs`), and what it was made
//! from besides its box. A box kept from the last update
//! (`LayoutBox::kept`) whose values below are unchanged keeps its part
//! in place (`paint_partial.rs`); any other is painted again.

use vapors_dom::NodeId;
use vapors_scene::{Item, Scene};

use crate::layout::{BoxRect, LayoutBox};
use crate::painted::Painted;
use crate::painting::Painting;
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
    pub(crate) outline: Scene,
    /// Its scroll thumbs' items, painted again on every update that
    /// paints.
    pub(crate) thumb: Scene,
}

/// What one step put into a paint; its items name ids its paint's
/// slabs hold while the part is kept.
#[derive(Default)]
pub(crate) struct Part {
    pub(crate) scene: Scene,
    pub(crate) hits: Vec<(NodeId, BoxRect)>,
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
    /// The items `step`, one of this box's steps, paints: its own
    /// part's, its outline or its thumbs.
    pub(crate) fn scene_of(&self, step: crate::paint_order::Step) -> &Scene {
        match step {
            crate::paint_order::Step::Box(_) => &self.own.scene,
            crate::paint_order::Step::Thumb(_) => &self.thumb,
            crate::paint_order::Step::Outline(_) => &self.outline,
        }
    }

    /// How many scene items this box's own step and outline hold.
    pub(crate) fn items(&self) -> usize {
        self.own.scene.items().len() + self.outline.items().len()
    }

    /// Each scene item this box's paint holds: its own step's, its
    /// outline's, then its thumbs'.
    pub(crate) fn all_items(&self) -> impl Iterator<Item = &Item> {
        let outline = self.outline.items().iter();
        let own = self.own.scene.items().iter();
        own.chain(outline).chain(self.thumb.items())
    }

    /// Frees the slab ids this box's paint names and removes its
    /// entries from `painted`'s maps, as it is dropped.
    pub(crate) fn drop_from(&self, painted: &mut Painted) {
        painted.slabs.free(self.own.scene.items());
        painted.slabs.free(self.outline.items());
        self.own.leave(painted);
    }
}

impl Part {
    /// What `painting` holds but its slabs, leaving it empty.
    pub(crate) fn take(painting: &mut Painting) -> Self {
        use std::mem::take;
        Self {
            scene: take(&mut painting.scene),
            hits: take(&mut painting.hits),
            cursors: painting.cursors.drain().collect(),
            visible: painting.visible.drain().collect(),
            rects: painting.rects.drain().collect(),
            ranges: painting.ranges.drain().collect(),
            text_origins: painting.text_origins.drain().collect(),
        }
    }

    /// Puts this part's entries into `painted`'s maps.
    pub(crate) fn enter(&self, painted: &mut Painted) {
        painted.cursors.extend(self.cursors.iter().copied());
        painted.visible.extend(self.visible.iter().copied());
        painted.rects.extend(self.rects.iter().copied());
        painted.ranges.extend(self.ranges.iter().copied());
        painted
            .text_origins
            .extend(self.text_origins.iter().copied());
    }

    /// Removes this part's entries from `painted`'s maps.
    fn leave(&self, painted: &mut Painted) {
        removed(&mut painted.cursors, &self.cursors);
        removed(&mut painted.rects, &self.rects);
        removed(&mut painted.ranges, &self.ranges);
        removed(&mut painted.text_origins, &self.text_origins);
        for node in &self.visible {
            painted.visible.remove(node);
        }
    }
}

/// Removes the node of each of `entries` from `map`.
fn removed<V, W>(map: &mut crate::node_map::NodeMap<V>, entries: &[(NodeId, W)]) {
    for (node, _) in entries {
        map.remove(node);
    }
}
