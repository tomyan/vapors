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

use vapors_app::{ElementsApp, FrameCounts, FrameTiming, SurfaceHandle};
use vapors_elements::Document;
use vapors_platform::{App, Event, Frame, Next, Platform, Start};
use vapors_scene3d::{CameraHandle, Meshes, Projection, Scene};
use vapors_scene3d_gpu::SceneRenderer;
use vapors_signals::Signal;
use vapors_text::FontSystem;

include!(concat!(env!("OUT_DIR"), "/wisp.rs"));

mod clock;
pub mod confirm;
pub mod parts;
pub mod schedule;
pub mod script;
mod shapes;

use clock::Clock;

use one_scene::{OneScene as Page, OneSceneProps};

/// The example's size in device pixels at scale 1: its window's, and the
/// judged run's target.
pub const SIZE: (u32, u32) = (1100, 720);

/// The example as an app: the page in an `ElementsApp`, its scene drawn
/// by a `SceneRenderer` on the surface named `view`.
pub struct OneScene {
    app: ElementsApp,
    document: Rc<RefCell<Document>>,
    scene: Rc<RefCell<Scene>>,
    clock: Clock,
    _page: Page,
}

impl OneScene {
    /// The example, mounted; its surface animates from the first frame,
    /// for `limit` frames or for ever.
    ///
    /// # Errors
    /// The renderer cannot be mounted, or the geometry is refused.
    pub fn new(limit: Option<u64>) -> Result<Self, String> {
        let document = Rc::new(RefCell::new(Document::new(FontSystem::new())));
        let mut app = ElementsApp::new(Rc::clone(&document));
        let scene = Rc::new(RefCell::new(Scene::new()));
        let meshes = Rc::new(RefCell::new(Meshes::new()));
        let surface = mount_renderer(&mut app, &scene, &meshes)?;
        let clock = Clock::new(surface.clone(), limit);
        let props = props(&scene, &meshes, surface, (clock.time, clock.frames))?;
        let body = document.borrow().body();
        let page = Page::mount(&document, body, props);
        Ok(Self {
            app,
            document,
            scene,
            clock,
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

    /// Render passes its last frame began.
    #[must_use]
    pub fn render_passes(&self) -> u64 {
        self.app.render_passes()
    }

    /// Its frames' times, the GPU's read back by the frame after.
    #[must_use]
    pub fn frame_timing(&self) -> FrameTiming {
        self.app.frame_timing()
    }

    /// Turns timing its frames on the GPU on or off; off by default.
    pub fn time_frames(&mut self, on: bool) {
        self.app.time_frames(on);
    }
}

/// A `SceneRenderer` of `scene` on the surface `view`, animating.
fn mount_renderer(
    app: &mut ElementsApp,
    scene: &Rc<RefCell<Scene>>,
    meshes: &Rc<RefCell<Meshes>>,
) -> Result<SurfaceHandle, String> {
    let renderer = SceneRenderer::new(Rc::clone(scene), Rc::clone(meshes));
    let surface = app
        .mount("view", Box::new(renderer))
        .map_err(|e| e.to_string())?;
    surface.request_animation().map_err(|e| e.to_string())?;
    Ok(surface)
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
        if let Err(error) = self.clock.tick(frame.now) {
            vapors_platform::report_error(&error);
            return Next::Exit;
        }
        self.app.frame(frame)
    }

    fn start(&mut self, start: &Start<'_>) -> Result<(), String> {
        self.app.start(start)
    }

    fn drawn(&mut self, platform: &Platform) {
        self.app.drawn(platform);
    }
}

/// Runs the example in a window (natively) or the page (on the web),
/// for `limit` frames or for ever.
///
/// # Errors
/// It cannot be made, or the window's loop failed.
pub fn run(limit: Option<u64>) -> Result<(), String> {
    vapors_platform::app_draws_open();
    vapors_platform::hide_status_line();
    let options = vapors_platform::WindowOptions {
        app_name: "vapors one scene".into(),
        title: "Vapors / One scene".into(),
        width: SIZE.0,
        height: SIZE.1,
    };
    vapors_platform::run(options, OneScene::new(limit)?).map_err(|error| error.to_string())
}
