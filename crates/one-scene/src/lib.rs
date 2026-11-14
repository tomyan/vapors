//! The one-scene example (`docs/perf-finish-line.md`, property 9): HTML
//! over a 3D scene in one document, CSS reaching the scene's meshes.
//! A HUD of buttons sits over a `gpu-surface`; three cubes turn on a
//! ground plane; a click on a light toggles its cube's `lit` class, and
//! the page's own time signal hides a cube and switches a colour token
//! (`schedule.rs`). One component, two hosts: the web demos page
//! (`?demo=one-scene`) runs it on the real clock, and `vapors-perf`'s
//! judged run on the frame clock, driven by [`script`].

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::{ElementsApp, FrameCounts, SurfaceHandle};
use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Instant, Next, Platform};
use vapors_scene3d::{CameraHandle, Meshes, Projection, Scene};
use vapors_scene3d_gpu::SceneRenderer;
use vapors_signals::Signal;
use vapors_text::FontSystem;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

pub mod parts;
pub mod schedule;
pub mod script;
mod shapes;

use one_scene::{OneScene as Page, OneSceneProps};

/// The example as an app: the page in an `ElementsApp`, its scene drawn
/// by a `SceneRenderer` on the surface named `view`.
pub struct OneScene {
    app: ElementsApp,
    document: Rc<RefCell<Document>>,
    scene: Rc<RefCell<Scene>>,
    time: Signal<Duration>,
    frames: Signal<u64>,
    started: Option<Instant>,
    _page: Page,
}

impl OneScene {
    /// The example, mounted; its surface animates from the first frame.
    ///
    /// # Errors
    /// The renderer cannot be mounted, or the geometry is refused.
    pub fn new() -> Result<Self, String> {
        let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
        let mut app = ElementsApp::new(Rc::clone(&document));
        let scene = Rc::new(RefCell::new(Scene::new()));
        let meshes = Rc::new(RefCell::new(Meshes::new()));
        let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes));
        let surface = app
            .mount("view", Box::new(renderer))
            .map_err(|e| e.to_string())?;
        surface.request_animation().map_err(|e| e.to_string())?;
        let (time, frames) = (Signal::new(Duration::ZERO), Signal::new(0));
        let props = props(&scene, &meshes, surface, (time, frames))?;
        let body = document.borrow().body();
        let page = Page::mount(&document, body, props);
        Ok(Self {
            app,
            document,
            scene,
            time,
            frames,
            started: None,
            _page: page,
        })
    }

    /// The page's document.
    #[must_use]
    pub fn document(&self) -> &Rc<RefCell<Document>> {
        &self.document
    }

    /// The scene the page's components build.
    #[must_use]
    pub fn scene(&self) -> &Rc<RefCell<Scene>> {
        &self.scene
    }

    /// The last frame's counts, the scene's work under `surfaces`.
    #[must_use]
    pub fn frame_counts(&self) -> FrameCounts {
        self.app.frame_counts()
    }

    /// The app it runs in, for its timings.
    pub fn elements_app(&mut self) -> &mut ElementsApp {
        &mut self.app
    }
}

/// The page's props: the scene, its geometry, a perspective camera.
fn props(
    scene: &Rc<RefCell<Scene>>,
    meshes: &Rc<RefCell<Meshes>>,
    surface: SurfaceHandle,
    (time, frames): (Signal<Duration>, Signal<u64>),
) -> Result<OneSceneProps, String> {
    let mut store = meshes.borrow_mut();
    let lens = Projection::perspective(0.9, 0.1, 100.0).map_err(|e| e.to_string())?;
    Ok(OneSceneProps {
        scene: Some(Rc::clone(scene)),
        surface: Some(surface),
        cube: Some(shapes::cube(&mut store).map_err(|e| e.to_string())?),
        ground: Some(shapes::ground(&mut store).map_err(|e| e.to_string())?),
        eye: Some(CameraHandle::new()),
        lens: Some(lens),
        time,
        frames,
    })
}

impl App for OneScene {
    fn event(&mut self, event: Event, platform: &Platform) {
        self.app.event(event, platform);
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        let started = *self.started.get_or_insert(frame.now);
        self.time.set(frame.now.duration_since(started));
        self.frames.update(|n| *n += 1);
        self.app.frame(frame)
    }

    fn drawn(&mut self, platform: &Platform) {
        self.app.drawn(platform);
    }
}

/// Runs the example in a window (natively) or the page (on the web).
///
/// # Errors
/// It cannot be made, or the window's loop failed.
pub fn run() -> Result<(), String> {
    vapors_platform::app_draws_open();
    vapors_platform::hide_status_line();
    let options = vapors_platform::WindowOptions {
        app_name: "vapors one scene".into(),
        title: "Vapors / One scene".into(),
        width: 1100,
        height: 720,
    };
    vapors_platform::run(options, OneScene::new()?).map_err(|error| error.to_string())
}
