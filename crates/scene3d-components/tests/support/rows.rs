//! The rows page (`pages/rows_page.wisp`): meshes in a keyed `{for}`,
//! mounted on a [`Stage`].

use super::rows_page::{RowsPage, RowsPageProps};
use super::{Stage, square};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_scene3d::Scene;
use vapors_signals::Signal;

pub struct RowsRig {
    pub scene: Rc<RefCell<Scene>>,
    pub rows: Signal<Vec<i32>>,
    pub page: RowsPage,
    pub stage: Stage,
}

impl RowsRig {
    pub fn new(rows: Vec<i32>) -> Self {
        let stage = Stage::new();
        let rows = Signal::new(rows);
        let props = RowsPageProps {
            scene: Some(Rc::clone(&stage.scene)),
            surface: Some(stage.surface.clone()),
            square: Some(square(&stage.meshes)),
            rows,
        };
        let body = stage.doc.borrow().body();
        let page = RowsPage::mount(&stage.doc, body, props);
        Self {
            scene: Rc::clone(&stage.scene),
            rows,
            page,
            stage,
        }
    }
}
