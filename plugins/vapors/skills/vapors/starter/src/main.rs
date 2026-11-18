//! The page: a box turning in a 3D scene, with HTML over it. `page.wisp`
//! holds the view, the clock and the keys; this file builds what the
//! scene needs and runs the page.

mod shapes;
mod view;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_platform::{WindowOptions, app_draws_open, hide_status_line, report_error, run};
use vapors_scene3d::{CameraHandle, Meshes, Scene};
use vapors_scene3d_gpu::SceneRenderer;
use vapors_signals::Signal;
use vapors_text::FontSystem;

fn main() {
    if let Err(message) = start() {
        report_error(&message);
    }
}

fn start() -> Result<(), String> {
    let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
    let mut app = ElementsApp::new(Rc::clone(&document));
    let scene = Rc::new(RefCell::new(Scene::new()));
    let meshes = Rc::new(RefCell::new(Meshes::new()));
    // the scene is drawn on the page's <gpu-surface name="view">
    let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes));
    let surface = app
        .mount("view", Box::new(renderer))
        .map_err(|e| e.to_string())?;
    let shapes = shapes::Shapes::new(&mut meshes.borrow_mut()).map_err(|e| e.to_string())?;
    let props = page::PageProps {
        scene: Some(scene),
        surface: Some(surface),
        shapes: Some(shapes),
        eye: Some(CameraHandle::new()),
        lens: Some(Signal::new(view::lens()?).read_only()),
    };
    let body = document.borrow().body();
    // the page lives as long as the app: never dropped
    std::mem::forget(page::Page::mount(&document, body, props));
    // the page has no Open button, and the status line would cover it
    app_draws_open();
    hide_status_line();
    let options = WindowOptions {
        app_name: "hello".into(),
        title: "hello".into(),
        width: 960,
        height: 640,
    };
    run(options, app).map_err(|e| e.to_string())
}
