//! The test page (`pages/scene_page.wisp`) mounted in a document, with
//! its scene drawn by a `SceneRenderer` mounted on an `ElementsApp`.

#![allow(dead_code)] // each test file uses part of it

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

use scene_page::{ScenePage, ScenePageProps};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_scene3d::{Meshes, Scene, Transform, Vec3};
use vapors_scene3d_gpu::SceneRenderer;
use vapors_signals::Signal;
use vapors_text::FontSystem;

/// The page's signals, as the test writes them.
pub struct Signals {
    pub inner: Signal<Transform>,
    pub shown: Signal<bool>,
}

pub struct Rig {
    pub doc: Rc<RefCell<Document>>,
    pub app: ElementsApp,
    pub scene: Rc<RefCell<Scene>>,
    pub page: ScenePage,
    pub signals: Signals,
}

pub fn at(x: f32) -> Transform {
    Transform::from_translation(Vec3::new(x, 0.0, 0.0)).unwrap()
}

impl Rig {
    /// The page mounted: a group under the root holding a group.
    pub fn new() -> Self {
        let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
        let sheet = "html, body { margin: 0; background-color: #ffffff; }";
        doc.borrow_mut().add_stylesheet("app.css", sheet).unwrap();
        let mut app = ElementsApp::new(Rc::clone(&doc));
        let scene = Rc::new(RefCell::new(Scene::new()));
        let meshes = Rc::new(RefCell::new(Meshes::new()));
        let renderer = SceneRenderer::new(Rc::clone(&scene), meshes);
        let surface = app.mount("view", Box::new(renderer)).unwrap();
        let signals = Signals {
            inner: Signal::new(Transform::IDENTITY),
            shown: Signal::new(false),
        };
        let props = ScenePageProps {
            scene: Some(Rc::clone(&scene)),
            surface: Some(surface),
            outer: Signal::new(at(1.0)),
            inner: signals.inner,
            shown: signals.shown,
        };
        let body = doc.borrow().body();
        let page = ScenePage::mount(&doc, body, props);
        Self {
            doc,
            app,
            scene,
            page,
            signals,
        }
    }
}
