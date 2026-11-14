//! The test page (`pages/scene_page.wisp`) mounted in a document, with
//! its scene drawn by a `SceneRenderer` mounted on an `ElementsApp`.

#![allow(dead_code)] // each test file uses part of it

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

pub mod keyed;
pub mod rows;

use scene_page::{ScenePage, ScenePageProps};
use std::cell::RefCell;
use std::rc::Rc;
use vapors_app::{ElementsApp, SurfaceHandle};
use vapors_elements::Document;
use vapors_scene3d::{CameraHandle, Colour, Meshes, Projection, Scene, Transform, Vec3};
use vapors_scene3d_gpu::{RendererLog, SceneRenderer};
use vapors_signals::Signal;
use vapors_text::FontSystem;

/// The page's signals, as the test writes them.
pub struct Signals {
    pub back: Signal<Transform>,
    pub left: Signal<Transform>,
    pub blue: Signal<bool>,
    pub gone: Signal<bool>,
    pub shown: Signal<bool>,
}

/// The page is the first field, so it is dropped (and disposed) first,
/// while the app runs; a preference since its nodes' removal takes a
/// closed loop or an unmounted renderer as nothing to draw again.
pub struct Rig {
    pub page: ScenePage,
    pub doc: Rc<RefCell<Document>>,
    pub app: ElementsApp,
    pub scene: Rc<RefCell<Scene>>,
    pub meshes: Rc<RefCell<Meshes>>,
    pub surface: SurfaceHandle,
    pub log: Rc<RendererLog>,
    pub signals: Signals,
}

pub fn red() -> Colour {
    Colour::srgb(1.0, 0.0, 0.0).unwrap()
}

pub fn at(x: f32) -> Transform {
    Transform::from_translation(Vec3::new(x, 0.0, 0.0)).unwrap()
}

/// A document with a `SceneRenderer` mounted on the surface named
/// `view`, before any page is.
pub struct Stage {
    pub doc: Rc<RefCell<Document>>,
    pub app: ElementsApp,
    pub scene: Rc<RefCell<Scene>>,
    pub meshes: Rc<RefCell<Meshes>>,
    pub surface: SurfaceHandle,
    /// The renderer's log: whether it was dropped.
    pub log: Rc<RendererLog>,
}

impl Stage {
    pub fn new() -> Self {
        let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
        let sheet = "html, body { margin: 0; background-color: #ffffff; }";
        doc.borrow_mut().add_stylesheet("app.css", sheet).unwrap();
        let mut app = ElementsApp::new(Rc::clone(&doc));
        let scene = Rc::new(RefCell::new(Scene::new()));
        let meshes = Rc::new(RefCell::new(Meshes::new()));
        let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes));
        let log = renderer.log();
        let surface = app.mount("view", Box::new(renderer)).unwrap();
        Self {
            doc,
            app,
            scene,
            meshes,
            surface,
            log,
        }
    }
}

impl Rig {
    /// The page mounted: a camera 5 units back looking at a red square
    /// 1 unit wide at the origin, through an orthographic view 4 high.
    pub fn new() -> Self {
        let Stage {
            doc,
            app,
            scene,
            meshes,
            surface,
            log,
        } = Stage::new();
        let square = square(&meshes);
        let signals = Signals {
            back: Signal::new(Transform::from_translation(Vec3::new(0.0, 0.0, 5.0)).unwrap()),
            left: Signal::new(Transform::IDENTITY),
            blue: Signal::new(false),
            gone: Signal::new(false),
            shown: Signal::new(false),
        };
        let eye = CameraHandle::new();
        let props = ScenePageProps {
            scene: Some(Rc::clone(&scene)),
            surface: Some(surface.clone()),
            square: Some(square),
            eye: Some(eye),
            lens: Some(Projection::orthographic(4.0, 0.1, 100.0).unwrap()),
            back: signals.back,
            left: signals.left,
            blue: signals.blue,
            gone: signals.gone,
            right: Signal::new(at(1.0)),
            shown: signals.shown,
        };
        let body = doc.borrow().body();
        let page = ScenePage::mount(&doc, body, props);
        Self {
            page,
            doc,
            app,
            scene,
            meshes,
            surface,
            log,
            signals,
        }
    }
}

/// A square 1 unit wide in the XY plane, centred on the origin.
pub fn square(meshes: &Rc<RefCell<Meshes>>) -> vapors_scene3d::GeometryHandle {
    let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let positions = corners.map(|(x, y)| Vec3::new(x, y, 0.0)).to_vec();
    meshes
        .borrow_mut()
        .geometry(positions, vec![0, 1, 2, 0, 2, 3], vec![1.0; 4])
        .unwrap()
}
