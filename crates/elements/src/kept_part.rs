//! The paint one step of a kept box put in (`kept_box.rs`): its scene,
//! hits and visible elements, which nearly every box has, and its
//! rarer entries boxed apart.

use vapors_dom::NodeId;
use vapors_scene::Scene;

use crate::layout::BoxRect;
use crate::painted::Painted;
use crate::painting::Painting;
use crate::style::Cursor;

/// What one step put into a paint; its items name ids its paint's
/// slabs hold while the part is kept.
#[derive(Default)]
pub(crate) struct Part {
    pub(crate) scene: Scene,
    pub(crate) hits: Vec<(NodeId, BoxRect)>,
    /// Every element paints its cursor, `auto` included.
    cursors: Vec<(NodeId, Cursor)>,
    visible: Vec<NodeId>,
    /// Its inline elements' rects, range tracks and text origins, when
    /// it has any: three empty lists are 72 bytes a slot.
    rare: Option<Box<Rare>>,
}

/// A part's entries few boxes have.
#[derive(Default)]
struct Rare {
    rects: Vec<(NodeId, BoxRect)>,
    ranges: Vec<(NodeId, crate::range_paint::Track)>,
    text_origins: Vec<(NodeId, [f32; 2])>,
}

/// What a part with no rarer entries reads for them.
static NO_RARE: Rare = Rare {
    rects: Vec::new(),
    ranges: Vec::new(),
    text_origins: Vec::new(),
};

impl Rare {
    /// What `painting` holds of these, leaving them empty; none when it
    /// holds none, with no map walked.
    fn take(painting: &mut Painting) -> Option<Box<Self>> {
        let none = painting.rects.is_empty()
            && painting.ranges.is_empty()
            && painting.text_origins.is_empty();
        (!none).then(|| {
            Box::new(Self {
                rects: painting.rects.drain().collect(),
                ranges: painting.ranges.drain().collect(),
                text_origins: painting.text_origins.drain().collect(),
            })
        })
    }
}

impl Part {
    /// What `painting` holds but its slabs, leaving it empty.
    pub(crate) fn take(painting: &mut Painting) -> Self {
        use std::mem::take;
        let rare = Rare::take(painting);
        Self {
            scene: take(&mut painting.scene),
            hits: take(&mut painting.hits),
            cursors: painting.cursors.drain().collect(),
            visible: painting.visible.drain().collect(),
            rare,
        }
    }

    /// Its rarer entries, empty when it has none.
    fn rare(&self) -> &Rare {
        self.rare.as_deref().unwrap_or(&NO_RARE)
    }

    /// Whether its rarer entries are boxed apart.
    #[cfg(test)]
    pub(crate) fn rare_boxed(&self) -> bool {
        self.rare.is_some()
    }

    /// Whether every position this part holds is in its scene and its
    /// hits: no inline element's border box, range track or text
    /// field's origin, which paint derives from more than the rect.
    pub(crate) fn positions_only_in_scene_and_hits(&self) -> bool {
        let rare = self.rare();
        rare.rects.is_empty() && rare.ranges.is_empty() && rare.text_origins.is_empty()
    }

    /// Puts this part's entries into `painted`'s maps, their positions
    /// in scroll frame `frame`: its inline elements' frames noted here,
    /// and a field's text origin's beside it, as its run's part is in
    /// the field's frame when the field scrolls; a range's own
    /// element's, by `kept_maps.rs`, which owns them and must agree.
    pub(crate) fn enter(&self, painted: &mut Painted, frame: usize) {
        let rare = self.rare();
        for (node, _) in &rare.rects {
            painted.node_frames.insert(*node, frame);
        }
        for (node, _) in &rare.ranges {
            assert_framed(painted, *node, frame);
        }
        painted.cursors.extend(self.cursors.iter().copied());
        painted.visible.extend(self.visible.iter().copied());
        painted.rects.extend(rare.rects.iter().copied());
        painted.ranges.extend(rare.ranges.iter().copied());
        let origins = rare.text_origins.iter();
        let framed = origins.map(|&(node, origin)| (node, (origin, frame)));
        painted.text_origins.extend(framed);
    }

    /// Removes this part's entries from `painted`'s maps.
    pub(crate) fn leave(&self, painted: &mut Painted) {
        let rare = self.rare();
        removed(&mut painted.cursors, &self.cursors);
        removed(&mut painted.rects, &rare.rects);
        removed(&mut painted.ranges, &rare.ranges);
        removed(&mut painted.text_origins, &rare.text_origins);
        for (node, _) in &rare.rects {
            painted.node_frames.remove(node);
        }
        for node in &self.visible {
            painted.visible.remove(node);
        }
    }
}

/// Panics, naming `node`, unless `painted` notes it in `frame`: the
/// element's box entries are put (`kept_maps.rs`) before its part is
/// entered (`paint_partial.rs`), in the frame its part is painted in.
fn assert_framed(painted: &Painted, node: NodeId, frame: usize) {
    let noted = painted.node_frames.get(&node).copied();
    assert!(
        noted == Some(frame),
        "{node:?}'s part is painted in scroll frame {frame}, but its box's entries note \
         {noted:?}: kept_maps.rs `fill` must run before the part is entered, with the \
         frame of the same clip chain (kept_clips.rs); reproduce with `cargo xtask test`"
    );
}

/// Removes the node of each of `entries` from `map`.
fn removed<V, W>(map: &mut crate::node_map::NodeMap<V>, entries: &[(NodeId, W)]) {
    for (node, _) in entries {
        map.remove(node);
    }
}
