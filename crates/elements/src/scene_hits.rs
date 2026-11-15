//! The hit sources of watched scene anchors (`Dom::watch_hits`,
//! docs/plans/css-for-3d.md C4): one per `scene` anchor and one per
//! surface name, dropped when the anchor is removed. What a hit test
//! asks of them is in `scene_hit`.

use std::cell::RefCell;
use std::collections::HashMap;

use vapors_dom::{HitSource, NodeId};

use crate::check::{describe, nodes};
use crate::document::Document;
use crate::error::ElementError;

/// The watched scenes' sources, and what their hit tests left to say.
#[derive(Default)]
pub(crate) struct SceneHits {
    /// Each watched scene anchor's surface name and source.
    pub(crate) sources: crate::node_map::NodeMap<(String, HitSource)>,
    /// The scene anchor watched on each surface name.
    pub(crate) surfaces: HashMap<String, NodeId>,
    /// The refusals hit tests met since the last update, which the next
    /// update returns: a hit test reads the tree, so it cannot refuse.
    pub(crate) refused: RefCell<Vec<ElementError>>,
    /// Whether a watched scene's hits moved under the pointer since the
    /// last update, so its hover is tested again.
    pub(crate) moved: bool,
}

impl Document {
    /// Keeps `source` for `anchor` on the surface named `surface`,
    /// refusing any node but a `scene`, a scene already watched and a
    /// surface already watched; the hover is tested again at the next
    /// update, as the pointer may rest on the surface.
    pub(crate) fn watch_scene_hits(&mut self, anchor: NodeId, surface: &str, source: HitSource) {
        if let Some(why) = self.unwatchable_hits(anchor, surface) {
            let element = describe(&self.arena, anchor);
            return self.refuse(anchor, format!("watch_hits on {element}: {why}"));
        }
        let hits = &mut self.scene_hits;
        hits.surfaces.insert(surface.to_owned(), anchor);
        hits.sources.insert(anchor, (surface.to_owned(), source));
        hits.moved = true;
        self.note_change();
    }

    /// Why `anchor` cannot be watched on `surface`; `None` when it can.
    fn unwatchable_hits(&self, anchor: NodeId, surface: &str) -> Option<String> {
        let element = self.arena.get(anchor, "watch_hits").element();
        if element.is_none_or(|element| element.tag != "scene") {
            return Some("only a 3D `scene` anchor answers hits over its surface; watch the scene's own anchor".to_owned());
        }
        if self.scene_hits.sources.contains_key(&anchor) {
            return Some("it is already watched, so two sources would answer one scene's hits; mount one scene component per anchor".to_owned());
        }
        self.scene_hits.surfaces.contains_key(surface).then(|| format!(
            "the gpu-surface {surface:?} already has a scene watched on it, and a hit there can have one answer; draw one scene per surface"
        ))
    }

    /// Notes that `anchor`'s scene may answer differently under its
    /// surface; the next update tests the hover again when the pointer
    /// is over that surface or one of the scene's anchors. Refused on a
    /// node whose hits are not watched.
    pub(crate) fn scene_hits_moved(&mut self, anchor: NodeId) {
        let Some((surface, _)) = self.scene_hits.sources.get(&anchor) else {
            let element = describe(&self.arena, anchor);
            let why = "its hits are not watched, so nothing tests them; call it with the anchor passed to watch_hits";
            return self.refuse(anchor, format!("hits_moved on {element}: {why}"));
        };
        let surface = surface.clone();
        if self.pointer_over_scene(anchor, &surface) {
            self.scene_hits.moved = true;
            self.note_change();
        }
    }

    /// Whether the hovered element is the surface named `surface` or in
    /// `anchor`'s subtree.
    fn pointer_over_scene(&self, anchor: NodeId, surface: &str) -> bool {
        let Some(hovered) = self.pointer.hovered.filter(|node| self.contains(*node)) else {
            return false;
        };
        crate::surfaces::name(&self.arena, hovered) == Some(surface)
            || self.chain(hovered).contains(&anchor)
    }

    /// Drops the sources of the scenes in `node`'s subtree, which is
    /// being removed.
    pub(crate) fn drop_scene_hits(&mut self, node: NodeId) {
        if self.scene_hits.sources.is_empty() {
            return;
        }
        for node in nodes(&self.arena, node) {
            if let Some((surface, _)) = self.scene_hits.sources.remove(&node) {
                self.scene_hits.surfaces.remove(&surface);
            }
        }
    }

    /// The refusals hit tests met since the last update, taken.
    pub(crate) fn take_hit_refusals(&mut self) -> Vec<ElementError> {
        self.scene_hits.refused.take()
    }

    /// Whether a hit test met a refusal the next update returns.
    pub(crate) fn hit_refusals_pending(&self) -> bool {
        !self.scene_hits.refused.borrow().is_empty()
    }
}
