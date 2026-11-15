//! What a hit test that lands on a watched scene's `gpu-surface` asks
//! the scene (docs/plans/css-for-3d.md C4): the point from the surface's
//! content box, its size, and which anchors take the pointer
//! (`scene_takes`). The anchor answered is the target. The surface is
//! the target over its border (not asked), where the scene answers
//! nothing, and while an anchor of the scene is not yet styled (the
//! update that styles it tests the hover again). A source's error, an
//! answer outside the scene or one that does not take the pointer, and a
//! surface whose box cannot be read leave the surface the target and are
//! refused by the next update.

use vapors_dom::{HitAsk, NodeId};

use crate::check::describe;
use crate::document::Document;
use crate::error::ElementError;
use crate::painted::Painted;
use crate::scene_takes::Takes;

impl Document {
    /// The anchor `point` hits through the scene watched on `element`,
    /// when `element` is a surface with one and the scene answers one.
    pub(crate) fn scene_hit(
        &self,
        painted: &Painted,
        element: NodeId,
        point: [f32; 2],
    ) -> Option<NodeId> {
        let name = crate::surfaces::name(&self.arena, element)?;
        let &scene = self.scene_hits.surfaces.get(name)?;
        let Some((origin, size)) = self.content_box(painted, element) else {
            return self.refuse_hit(scene, "its surface's box could not be read from the last update, so the point could not be placed on it");
        };
        let local = [point[0] - origin[0], point[1] - origin[1]];
        if !(0.0..=size[0]).contains(&local[0]) || !(0.0..=size[1]).contains(&local[1]) {
            return None; // over the surface's border: the surface's hit
        }
        self.ask_scene(scene, local, size)
    }

    /// `scene`'s answer at `point` on a content box of `size`, checked.
    fn ask_scene(&self, scene: NodeId, point: [f32; 2], size: [f32; 2]) -> Option<NodeId> {
        let takes = Takes::new(self, scene);
        let ask = HitAsk {
            point,
            size,
            takes: &|anchor| takes.ask(anchor),
        };
        let target = match self.scene_hits.source(scene)(ask) {
            Ok(answer) => answer.filter(|&anchor| takes.answered(anchor)),
            Err(error) => self.refuse_hit(scene, &format!("its hit source failed: {error}")),
        };
        // an answer reached by passing over an unstyled anchor is not
        // trusted; the update that styles it tests the hover again
        // (`restyle_scene`, `takes_changed`)
        if takes.unresolved() {
            return None;
        }
        target
    }

    /// Keeps a refusal of `scene`'s hit for the next update, once per
    /// message however often the pointer meets it; `None`.
    pub(crate) fn refuse_hit(&self, scene: NodeId, why: &str) -> Option<NodeId> {
        let element = describe(&self.arena, scene);
        let error = ElementError::Refused(format!("{element}: {why}; the surface was the target"));
        let mut refused = self.scene_hits.refused.borrow_mut();
        if !refused.contains(&error) {
            refused.push(error);
        }
        None
    }

    /// The top-left of `element`'s content box on the page and its size:
    /// its border box less its borders (a surface refuses padding).
    fn content_box(&self, painted: &Painted, element: NodeId) -> Option<([f32; 2], [f32; 2])> {
        let rect = painted.border_box(element)?;
        let used = self.kept.as_ref()?.used.get(&element)?;
        let [top, right, bottom, left] = used.decoration.border;
        let origin = [rect.x + left, rect.y + top];
        Some((
            origin,
            [rect.width - left - right, rect.height - top - bottom],
        ))
    }

    /// The cursor of `node`: a scene anchor's, else the element's as
    /// painted.
    pub(crate) fn cursor_of(&self, painted: &Painted, node: NodeId) -> crate::Cursor {
        let scene = self.kept.as_ref().and_then(|kept| kept.scene.get(&node));
        scene
            .map(|used| used.cursor)
            .or_else(|| painted.cursors.get(&node).copied())
            .unwrap_or(crate::Cursor::Default)
    }
}

#[cfg(test)]
#[path = "scene_hit_tests.rs"]
mod tests;
