//! The appearance sinks of watched mesh anchors (`Dom::watch_appearance`,
//! docs/plans/css-for-3d.md C3): each is called with its anchor's
//! appearance at the first restyle and whenever a restyle changes it,
//! and dropped when the anchor is removed.

use vapors_dom::{AppearanceSink, NodeId};

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
        if let Some(refusal) = refuse_translucent(used, self.element_styles(node)) {
            return self.unhonoured(node, vec![refusal]).collect();
        }
        let appearance = used.appearance();
        if old.map(|old| old.appearance()) == Some(appearance) {
            return Vec::new();
        }
        let sink = self.sinks.get_mut(&node).expect("checked above");
        let refused = sink(appearance).err();
        let describe = || describe(&self.arena, node);
        (refused.into_iter())
            .map(|error| {
                ElementError::Refused(format!(
                    "{}: its appearance {appearance:?} was not applied: {error}",
                    describe()
                ))
            })
            .collect()
    }
}
