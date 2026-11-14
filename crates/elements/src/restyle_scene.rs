//! The used style of a scene anchor in a restyle (docs/plans/css-for-3d.md
//! C2): its `SceneUsed`, kept in place of a box `Used`, so it builds no
//! box and paints nothing; or its refusals.

use vapors_css::property::scene::SceneTag;
use vapors_dom::NodeId;

use crate::document::Document;
use crate::style::{SceneContext, SceneUsed, USER_AGENT_SHEET, scene_used};

impl Document {
    /// Keeps `node`'s `SceneUsed`, or its refusals, when it is a scene
    /// anchor; whether it is one.
    pub(crate) fn keep_scene_used(&mut self, node: NodeId) -> bool {
        let Some(context) = self.scene_context(node) else {
            return false;
        };
        let found = scene_used(self.element_styles(node), &context);
        let refused = (found.as_ref().err())
            .map(|refusals| self.unhonoured(node, refusals.clone()).collect())
            .unwrap_or_default();
        let kept = self.kept.as_mut().expect("restyle sets it");
        kept.refused.insert(node, refused);
        kept.used.remove(&node);
        match found {
            Ok(used) => drop(kept.scene.insert(node, used)),
            Err(_) => drop(kept.scene.remove(&node)),
        }
        true
    }

    /// Where `node` is, when it is a scene anchor.
    fn scene_context(&self, node: NodeId) -> Option<SceneContext> {
        debug_assert_eq!(
            self.sheets.name(USER_AGENT_SHEET),
            crate::style::USER_AGENT_NAME,
            "the sheet at USER_AGENT_SHEET is not the user-agent sheet, so an author sheet would escape the scene set check; put the user-agent sheet first in Sheets::new (crates/elements/src/style/mod.rs). Reproduce with `cargo test -p vapors-elements --test scene_used`"
        );
        let element = self.arena.get(node, "restyle").element()?;
        Some(SceneContext {
            tag: SceneTag::from_tag(&element.tag)?,
            key: element.key,
            user_agent: USER_AGENT_SHEET,
        })
    }

    /// The `SceneUsed` the last update kept for `node`; `None` when it
    /// is no scene anchor (`scene`, `group`, `mesh`, `camera`, `decal`)
    /// or its own style was refused. A value refused at an ancestor and
    /// inherited here is kept: that update failed, naming the ancestor.
    #[must_use]
    pub fn scene_used(&self, node: NodeId) -> Option<SceneUsed> {
        self.kept.as_ref()?.scene.get(&node).copied()
    }
}
