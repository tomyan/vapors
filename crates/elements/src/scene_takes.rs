//! Whether an anchor takes the pointer, as a scene's hit source asks it
//! (`HitAsk::takes`): three answers. A styled anchor of the scene takes
//! it when its `pointer-events` is `auto` and its `visibility` is
//! `visible`. An anchor of the scene not styled since it was added is
//! unresolved until the next update: the hit is answered by the surface
//! and tested again then. Anything else (a stale id, a node outside the
//! scene, an element that is no scene anchor) is the source's mistake,
//! refused by the next update.

use std::cell::Cell;

use vapors_dom::NodeId;

use crate::document::Document;

/// One hit's questions to the document about `scene`'s anchors.
pub(crate) struct Takes<'a> {
    doc: &'a Document,
    scene: NodeId,
    unresolved: Cell<bool>,
}

impl<'a> Takes<'a> {
    pub(crate) fn new(doc: &'a Document, scene: NodeId) -> Self {
        Self {
            doc,
            scene,
            unresolved: Cell::new(false),
        }
    }

    /// The answer the source is given for `anchor`: `false` for an
    /// unresolved anchor (noted) and for a mistaken id (refused).
    pub(crate) fn ask(&self, anchor: NodeId) -> bool {
        match self.resolve(anchor) {
            Ok(Some(takes)) => takes,
            Ok(None) => {
                self.unresolved.set(true);
                false
            }
            Err(why) => self.doc.refuse_hit(self.scene, &why).is_some(),
        }
    }

    /// Whether the source's answer `anchor` stands: in the scene and
    /// taking the pointer; refused for the next update when it is not.
    pub(crate) fn answered(&self, anchor: NodeId) -> bool {
        let why = match self.resolve(anchor) {
            Ok(Some(true)) => return true,
            Ok(Some(false)) => format!(
                "its hit source answered {anchor:?}, whose anchor does not take the pointer (pointer-events: none, or not visible): pass over the nodes `takes` answers false for"
            ),
            Ok(None) => {
                self.unresolved.set(true);
                return false;
            }
            Err(why) => why,
        };
        self.doc.refuse_hit(self.scene, &why);
        false
    }

    /// Whether a question about an unstyled anchor left the hit
    /// unresolved.
    pub(crate) fn unresolved(&self) -> bool {
        self.unresolved.get()
    }

    /// `Some(takes)` for a styled anchor of the scene, `None` for one not
    /// yet styled, the refusal for anything else.
    fn resolve(&self, anchor: NodeId) -> Result<Option<bool>, String> {
        let doc = self.doc;
        let in_scene = doc.contains(anchor) && doc.chain(anchor).contains(&self.scene);
        if !in_scene || !is_scene_anchor(doc, anchor) {
            return Err(format!(
                "its hit source named {anchor:?}, which is not an anchor in its scene: answer and ask about anchors under the scene anchor only"
            ));
        }
        let kept = doc.kept.as_ref().and_then(|kept| kept.scene.get(&anchor));
        Ok(kept.map(|used| used.takes_pointer && used.visible))
    }
}

/// Whether `node` is a scene anchor (`scene`, `group`, `mesh`, `camera`, `light`, `decal`).
fn is_scene_anchor(doc: &Document, node: NodeId) -> bool {
    let element = doc.arena.get(node, "scene hit").element();
    element.is_some_and(|element| {
        vapors_css::property::scene::SceneTag::from_tag(&element.tag).is_some()
    })
}
