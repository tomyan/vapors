//! The appearance sinks of watched mesh anchors (`Dom::watch_appearance`,
//! docs/plans/css-for-3d.md C3): each is called with its anchor's
//! appearance at the first restyle and whenever a restyle changes it,
//! and dropped when the anchor is removed.

use vapors_dom::{Appearance, AppearanceSink, NodeId};

use crate::check::{describe, nodes};
use crate::document::Document;
use crate::error::ElementError;
use crate::style::{SceneUsed, refuse_translucent};

impl Document {
    /// Keeps `sink` for `anchor`, refusing any node but a `mesh`, and a
    /// mesh already watched.
    pub(crate) fn watch(&mut self, anchor: NodeId, sink: AppearanceSink) {
        if let Some(why) = self.unwatchable(anchor) {
            let element = describe(&self.arena, anchor);
            return self.refuse(anchor, format!("watch_appearance on {element}: {why}"));
        }
        self.note_change();
        self.dirty.element(anchor);
        // forget the kept appearance, so the next restyle reports it
        // even when the anchor was styled before the watch
        if let Some(kept) = self.kept.as_mut() {
            kept.scene.remove(&anchor);
        }
        self.sinks.insert(anchor, sink);
    }

    /// Why `anchor` cannot be watched; `None` when it can.
    fn unwatchable(&self, anchor: NodeId) -> Option<&'static str> {
        let element = self.arena.get(anchor, "watch_appearance").element();
        if element.is_none_or(|element| element.tag != "mesh") {
            return Some(
                "only a 3D scene `mesh` anchor has an appearance to report; watch the mesh's own anchor",
            );
        }
        self.sinks.contains_key(&anchor).then_some(
            "it is already watched, so two components would write one node's appearance; mount one component per anchor",
        )
    }

    /// Whether `node` is watched and an ancestor's style is refused: the
    /// mesh may inherit the refused value, so its sink hears nothing
    /// until the restyle that fixes the ancestor reaches it.
    pub(crate) fn held_back(&self, node: NodeId) -> bool {
        let kept = self.kept.as_ref().expect("restyle sets it");
        let refused = |ancestor: &NodeId| kept.refused.get(ancestor).is_some_and(|r| !r.is_empty());
        self.sinks.contains_key(&node) && self.chain(node).iter().skip(1).any(refused)
    }

    /// Keeps `node` as its sink last heard it: no `Used`, no refusal of
    /// its own, its kept appearance unchanged.
    pub(crate) fn hold(&mut self, node: NodeId) {
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.used.remove(&node);
        kept.refused.insert(node, Vec::new());
    }

    /// Drops the sinks in `node`'s subtree, which is being removed.
    pub(crate) fn drop_sinks(&mut self, node: NodeId) {
        if self.sinks.is_empty() {
            return;
        }
        for node in nodes(&self.arena, node) {
            self.sinks.remove(&node);
        }
    }

    /// Calls `node`'s sink, if it has one, when `used` gives another
    /// appearance than `old`; the refusals that earns.
    pub(crate) fn report_appearance(
        &mut self,
        node: NodeId,
        old: Option<SceneUsed>,
        used: &SceneUsed,
    ) -> Vec<ElementError> {
        if !self.sinks.contains_key(&node) {
            return Vec::new();
        }
        let translucent = refuse_translucent(used, self.element_styles(node));
        if !translucent.is_empty() {
            return self.unhonoured(node, translucent).collect();
        }
        let appearance = used.appearance();
        if old.map(|old| old.appearance()) == Some(appearance) {
            return Vec::new();
        }
        self.call_sink(node, appearance).into_iter().collect()
    }

    /// Calls `node`'s sink with `appearance`; its error, as a refusal.
    /// A refused appearance is reported again at the next update, which
    /// restyles the anchor for it.
    fn call_sink(&mut self, node: NodeId, appearance: Appearance) -> Option<ElementError> {
        let sink = self
            .sinks
            .get_mut(&node)
            .expect("report_appearance checked it");
        let error = sink(appearance).err()?;
        self.dirty.element(node);
        let element = describe(&self.arena, node);
        Some(ElementError::Refused(format!(
            "{element}: its appearance {appearance:?} was not applied: {error}"
        )))
    }
}
