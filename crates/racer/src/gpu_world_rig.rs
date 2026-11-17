//! The racer's world mounted as the racer mounts it (`TrackWorld`, one-scene
//! R3a2), on a test page holding only the `track` surface
//! (`tests/pages/racer_target.wisp`), drawn by an `ElementsApp`: for
//! `gpu_world_tests.rs` to compare with the golden pictures, and for the
//! racer's GPU tests to see the world without the HUD over it.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{ElementsApp, GpuWork, SurfaceSize};
use vapors_elements::Document;
use vapors_gpu_harness::{Gpu, read_rgba8};
use vapors_platform::{App, Frame, Instant};
use vapors_renderer::{Samples, clear};
use vapors_text::FontSystem;

use crate::app::world::TrackWorld;
use crate::model::{Camera, Race};

#[allow(
    dead_code,
    clippy::needless_pass_by_value,
    reason = "the generated page's code: the tests use part of its API, and a page whose props are plain values takes them by value as every page does"
)]
mod pages {
    include!(concat!(env!("OUT_DIR"), "/pages/wisp.rs"));
}
use pages::racer_target::{RacerTarget, RacerTargetProps};

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

/// The mounted world and what draws it. The world is the first field,
/// so its page is dropped first, while the app runs.
pub struct Rig {
    pub world: TrackWorld,
    _target: RacerTarget,
    app: ElementsApp,
    pub size: SurfaceSize,
}

impl Rig {
    /// The world at `size` (`LANDSCAPE` or `PORTRAIT`) and `samples`,
    /// showing `race` through `camera`.
    pub fn new(race: &Race, camera: &Camera, size: SurfaceSize, samples: Samples) -> Self {
        let props = RacerTargetProps {
            portrait: size == PORTRAIT,
            window: false,
        };
        Self::mount(props, (samples, race, camera), size)
    }

    /// The world filling a window of `size` at `samples`: what the racer
    /// at `samples` shows where no control of its HUD covers it.
    pub fn window(race: &Race, camera: &Camera, size: SurfaceSize, samples: Samples) -> Self {
        let props = RacerTargetProps {
            portrait: false,
            window: true,
        };
        Self::mount(props, (samples, race, camera), size)
    }

    fn mount(props: RacerTargetProps, shown: (Samples, &Race, &Camera), size: SurfaceSize) -> Self {
        let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
        let body = doc.borrow().body();
        let target = RacerTarget::mount(&doc, body, props);
        let mut app = ElementsApp::new(Rc::clone(&doc)).with_samples(shown.0);
        let world = TrackWorld::mount(&mut app, &doc, shown);
        Self {
            world,
            _target: target,
            app,
            size,
        }
    }

    /// One app frame into a fresh target: the surfaces' counted work and
    /// the frame's pixels.
    pub fn frame(&mut self, gpu: &Gpu) -> (GpuWork, Vec<u8>) {
        let SurfaceSize {
            width,
            height,
            scale,
        } = self.size;
        let texture = clear(
            &gpu.device,
            &gpu.queue,
            width,
            height,
            vapors_scene::Color::default(),
        );
        self.app.frame(&Frame {
            adapter: &gpu.adapter,
            device: &gpu.device,
            queue: &gpu.queue,
            texture: &texture,
            index: 0,
            scale_factor: f64::from(scale),
            now: Instant::now(),
            resumed: false,
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
