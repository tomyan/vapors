//! The keyed page (`pages/keyed_page.wisp`): scene components carrying
//! `class` and `class:`, mounted on a [`Stage`].

use super::keyed_page::{KeyedPage, KeyedPageProps};
use super::{Stage, square};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, UpdateCounts, Viewport};
use vapors_scene3d::Scene;
use vapors_signals::Signal;

pub struct KeyedRig {
    pub doc: Rc<RefCell<Document>>,
    pub scene: Rc<RefCell<Scene>>,
    pub braking: Signal<bool>,
    pub page: KeyedPage,
    _stage: Stage,
}

impl KeyedRig {
    pub fn new() -> Self {
        let stage = Stage::new();
        let braking = Signal::new(false);
        let props = KeyedPageProps {
            scene: Some(Rc::clone(&stage.scene)),
            surface: Some(stage.surface.clone()),
            square: Some(square(&stage.meshes)),
            braking,
        };
        let body = stage.doc.borrow().body();
        let page = KeyedPage::mount(&stage.doc, body, props);
        Self {
            doc: Rc::clone(&stage.doc),
            scene: Rc::clone(&stage.scene),
            braking,
            page,
            _stage: stage,
        }
    }

    /// Updates the document; what the update did.
    pub fn update(&self) -> UpdateCounts {
        let viewport = Viewport {
            width: 64.0,
            height: 64.0,
            scale: 1.0,
        };
        let mut doc = self.doc.borrow_mut();
        doc.update(viewport).unwrap();
        doc.update_counts()
    }

    /// Each scene node's anchor, in document order.
    pub fn anchors(&self) -> Vec<NodeId> {
        let doc = self.doc.borrow();
        anchors_under(&doc, doc.body())
    }
}

/// The elements under `node` holding a scene node.
pub fn anchors_under(doc: &Document, node: NodeId) -> Vec<NodeId> {
    let held = doc.current_attribute(node, "data-scene3d-node").is_some();
    let below = doc
        .children(node)
        .into_iter()
        .flat_map(|child| anchors_under(doc, child));
    held.then_some(node).into_iter().chain(below).collect()
}
