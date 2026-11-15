//! What a `<Scene>` tells the document about hits on its surface
//! (docs/plans/css-for-3d.md C4): which mesh node each mesh anchor
//! holds, so a picked node is answered as its anchor, and when a node
//! moved, so the hover under a resting pointer is tested again
//! (`Dom::hits_moved`). The document is held weakly: the source the
//! document keeps holds this, and a strong hold would keep both alive.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use vapors_dom::{Dom, HitAsk, HitSource, NodeId as ElementId};
use vapors_scene3d::{NodeId, PickAt, Scene, pick};
use vapors_signals::on_cleanup;

/// One scene's mesh anchors and the document they are in.
pub(crate) struct SceneHits {
    pub(crate) scene_anchor: ElementId,
    dom: Weak<RefCell<dyn Dom>>,
    anchors: RefCell<HashMap<NodeId, ElementId>>,
}

impl SceneHits {
    pub(crate) fn new(dom: &Rc<RefCell<dyn Dom>>, scene_anchor: ElementId) -> Rc<Self> {
        Rc::new(Self {
            scene_anchor,
            dom: Rc::downgrade(dom),
            anchors: RefCell::default(),
        })
    }

    /// Answers `node` as `anchor` until the current owner is disposed.
    pub(crate) fn name(self: &Rc<Self>, node: NodeId, anchor: ElementId) {
        self.anchors.borrow_mut().insert(node, anchor);
        let hits = Rc::clone(self);
        on_cleanup(move || {
            hits.anchors.borrow_mut().remove(&node);
        });
    }

    /// The anchor of `node`, when it is a mesh mounted by `<Mesh>`.
    pub(crate) fn anchor(&self, node: NodeId) -> Option<ElementId> {
        self.anchors.borrow().get(&node).copied()
    }

    /// Tells the document a node moved; nothing once the document is
    /// gone, as nothing is hovered then.
    pub(crate) fn moved(&self) {
        if let Some(dom) = self.dom.upgrade() {
            dom.borrow_mut().hits_moved(self.scene_anchor);
        }
    }
}

/// The source a `<Scene>` gives the document: the mesh `pick` finds
/// through the camera, as its anchor. A mesh whose anchor does not take
/// the pointer is passed over. A mesh no `<Mesh>` mounted (added to the
/// scene by the application) has no anchor: it blocks the pointer as it
/// is drawn, and a pick of it is answered as nothing, so the surface is
/// the target. A refused pick is the source's error, naming why.
pub(crate) fn hit_source(scene: Rc<RefCell<Scene>>, hits: Rc<SceneHits>) -> HitSource {
    Box::new(move |ask: HitAsk<'_>| {
        let at = PickAt {
            point: ask.point,
            size: ask.size,
        };
        let skip = |node| hits.anchor(node).is_some_and(|anchor| !(ask.takes)(anchor));
        let picked = pick(&mut scene.borrow_mut(), at, skip);
        let picked = picked.map_err(|error| format!("<Scene>: {error}"))?;
        Ok(picked.and_then(|node| hits.anchor(node)))
    })
}
