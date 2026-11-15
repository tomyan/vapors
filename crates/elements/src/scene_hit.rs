//! What a hit test that lands on a watched scene's `gpu-surface` asks
//! the scene (docs/plans/css-for-3d.md C4): the point from the surface's
//! content box, its size, and which anchors take the pointer. The anchor
//! answered is the target; nothing answered leaves the surface the
//! target; an error, or an anchor outside the scene, leaves the surface
//! the target and is refused by the next update.

use vapors_dom::{HitAsk, NodeId};

use crate::check::describe;
use crate::document::Document;
use crate::error::ElementError;
use crate::painted::Painted;

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
        let (origin, size) = self.content_box(painted, element)?;
        let takes = |anchor: NodeId| self.scene_takes_pointer(anchor);
        let ask = HitAsk {
            point: [point[0] - origin[0], point[1] - origin[1]],
            size,
            takes: &takes,
        };
        let (_, source) = &self.scene_hits.sources[&scene];
        match source(ask) {
            Ok(answer) => answer.filter(|&anchor| self.in_scene(scene, anchor)),
            Err(error) => self.refuse_hit(scene, &format!("its hit source failed: {error}")),
        }
    }

    /// Whether `anchor` is in `scene`'s subtree; refused for the next
    /// update when it is not.
    fn in_scene(&self, scene: NodeId, anchor: NodeId) -> bool {
        if self.contains(anchor) && self.chain(anchor).contains(&scene) {
            return true;
        }
        let why = format!(
            "its hit source answered {anchor:?}, which is not in its scene: answer an anchor under the scene anchor, or None"
        );
        self.refuse_hit(scene, &why);
        false
    }

    /// Keeps a refusal of `scene`'s hit for the next update; `None`.
    fn refuse_hit(&self, scene: NodeId, why: &str) -> Option<NodeId> {
        let element = describe(&self.arena, scene);
        let error = ElementError::Refused(format!("{element}: {why}; the surface was the target"));
        self.scene_hits.refused.borrow_mut().push(error);
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

    /// Whether scene anchor `anchor` takes the pointer: `pointer-events:
    /// auto` and `visibility: visible`.
    fn scene_takes_pointer(&self, anchor: NodeId) -> bool {
        let kept = self.kept.as_ref().and_then(|kept| kept.scene.get(&anchor));
        kept.is_some_and(|used| used.takes_pointer && used.visible)
    }

    /// The cursor of what `point` hits: a scene anchor's, else the
    /// element's as painted.
    pub(crate) fn cursor_of(&self, painted: &Painted, node: NodeId) -> crate::Cursor {
        let scene = self.kept.as_ref().and_then(|kept| kept.scene.get(&node));
        scene
            .map(|used| used.cursor)
            .or_else(|| painted.cursors.get(&node).copied())
            .unwrap_or(crate::Cursor::Default)
    }
}
