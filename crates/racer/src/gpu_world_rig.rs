//! The racer's world page (`tests/pages/racer_world.wisp`, one-scene
//! R3a1) mounted in a document, its scene drawn by a `SceneRenderer` on
//! the `track` surface of an `ElementsApp`; and the Track renderer drawn
//! at the same race and camera, for `gpu_world_tests.rs` to compare.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, GpuWork, SurfaceSize};
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::clear;
use vapors_scene3d::{CameraHandle, Meshes, Scene};
use vapors_scene3d_gpu::SceneRenderer;
use vapors_signals::Signal;
use vapors_text::FontSystem;

use crate::gpu::{Scene as TrackScene, Track, TrackLog};
use crate::model::{Camera, Race};
use crate::world::{self, camera_transform, car_transform};

#[allow(
    dead_code,
    reason = "the generated page's API, which the tests use part of"
)]
mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages/wisp.rs"));
}
use pages::racer_world::{RacerWorld, RacerWorldProps};

use crate::gpu_ground_material_target as target;

/// The page's surface, as its CSS sizes it: landscape, or with
/// `portrait` turned.
pub const LANDSCAPE: SurfaceSize = SurfaceSize {
    width: 320,
    height: 240,
    scale: 1.0,
};
pub const PORTRAIT: SurfaceSize = SurfaceSize {
    width: 240,
    height: 320,
    scale: 1.0,
};

/// The mounted page and what draws it. The page is the first field, so
/// it is dropped first, while the app runs.
pub struct Rig {
    _page: RacerWorld,
    app: ElementsApp,
    pub scene: Rc<RefCell<Scene>>,
    pub size: SurfaceSize,
}

impl Rig {
    /// The page mounted at `size` (`LANDSCAPE` or `PORTRAIT`) with the
    /// racer's parts, the camera placed by `camera` over `race`.
    pub fn new(race: &Race, camera: &Camera, size: SurfaceSize) -> Self {
        let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
        let mut app = ElementsApp::new(Rc::clone(&doc));
        let scene = Rc::new(RefCell::new(Scene::new()));
        let meshes = Rc::new(RefCell::new(Meshes::new()));
        let renderer = SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes));
        let surface = app.mount("track", Box::new(renderer)).unwrap();
        let mut held = meshes.borrow_mut();
        let ground_material = held.material(crate::ground_material()).expect("the ground");
        let (ground, parts) = (
            world::ground_geometry(&mut held),
            world::part_geometry(&mut held),
        );
        drop(held);
        let props = RacerWorldProps {
            scene: Some(Rc::clone(&scene)),
            surface: Some(surface),
            eye: Some(CameraHandle::new()),
            lens: Some(Signal::new(world::lens(camera).unwrap())),
            portrait: size == PORTRAIT,
            look: Signal::new(camera_transform(camera, race).unwrap()),
            world: Signal::new(world::racers_world()),
            car: Signal::new(car_transform(race).unwrap()),
            ground: Some(ground.unwrap()),
            ground_material: Some(ground_material),
            parts: parts.unwrap(),
        };
        let body = doc.borrow().body();
        let page = RacerWorld::mount(&doc, body, props);
        Self {
            _page: page,
            app,
            scene,
            size,
        }
    }

    /// One app frame into a fresh target: the surfaces' counted work and
    /// the frame's pixels.
    pub fn frame(&mut self, gpu: &Gpu) -> (GpuWork, Vec<u8>) {
        let texture = clear(
            &gpu.device,
            &gpu.queue,
            self.size.width,
            self.size.height,
            vapors_scene::Color::default(),
        );
        self.app.frame(&Frame {
            adapter: &gpu.adapter,
            device: &gpu.device,
            queue: &gpu.queue,
            texture: &texture,
            index: 0,
            scale_factor: 1.0,
            now: Instant::now(),
        });
        let pixels = read_rgba8(&gpu.device, &gpu.queue, &texture);
        (self.app.frame_counts().surfaces, pixels)
    }
}

/// `size`'s width and height as `f32`, exactly.
pub fn size_f32(size: SurfaceSize) -> (f32, f32) {
    let exact = |side: u32| f32::from(u16::try_from(side).expect("a test target's side"));
    (exact(size.width), exact(size.height))
}

/// The Track renderer's picture of `race` through `camera`, at `size`.
pub fn by_track(gpu: &Gpu, (race, camera): (&Race, &Camera), size: SurfaceSize) -> Vec<u8> {
    let scene = TrackScene {
        race: race.clone(),
        camera: *camera,
        brake: 0.0,
    };
    let mut track = Track::new(Rc::new(RefCell::new(scene)), Rc::new(TrackLog::default()));
    target::mount_at(&mut track, gpu, size);
    target::drawn_at(&mut track, gpu, size)
}
