//! The racer's world on the HUD's `gpu-surface` named `track`
//! (one-scene 2.2, R3a2): a `SceneRenderer` drawing the world component
//! (`ui/racer_world.wisp`), mounted beside the HUD. The racer writes the
//! camera, its lens and the car before each frame, each only when it
//! changed; the scene components draw the surface again when one does,
//! so a paused racer whose camera has settled renders nothing.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::ElementsApp;
use vapors_elements::Document;
use vapors_renderer::Samples;
use vapors_scene3d::{CameraHandle, Meshes, Projection, Scene, Transform};
use vapors_scene3d_gpu::{RendererLog, SceneRenderer};
use vapors_signals::Signal;

use crate::model::{Camera, Race};
use crate::racer_world::{RacerWorld, RacerWorldProps};
use crate::world::{self, camera_transform, car_transform, lens};

/// The `name` of the HUD's `gpu-surface`.
const NAME: &str = "track";

pub(crate) struct TrackWorld {
    look: Signal<Transform>,
    lens: Signal<Projection>,
    car: Signal<Transform>,
    // RELAXED: read only by the racer's GPU tests until the inspector
    // reports renders.
    #[cfg_attr(not(test), allow(dead_code))]
    log: Rc<RendererLog>,
    #[cfg(test)]
    scene: Rc<RefCell<Scene>>,
    _page: RacerWorld,
}

impl TrackWorld {
    /// Mounts a `SceneRenderer` at `samples` on `app`'s `track` surface
    /// and the world component in `document`'s body, showing `race`
    /// through `camera`.
    ///
    /// # Panics
    /// When `app` already has a renderer named `track`, or the world's
    /// geometry or ground material is refused: each is fixed data, held
    /// by the tests in `world.rs` and `ground_material.rs`.
    pub(crate) fn mount(
        app: &mut ElementsApp,
        document: &Rc<RefCell<Document>>,
        (samples, race, camera): (Samples, &Race, &Camera),
    ) -> Self {
        let scene = Rc::new(RefCell::new(Scene::new()));
        let meshes = Rc::new(RefCell::new(Meshes::new()));
        let renderer =
            SceneRenderer::new(Rc::clone(&scene), Rc::clone(&meshes)).with_samples(samples);
        let log = renderer.log();
        let surface = app
            .mount(NAME, Box::new(renderer))
            .expect("a new app has no renderers");
        let props = props(&meshes, (race, camera));
        let lens = Signal::new(lens(camera).expect("the racer's camera"));
        let (look, car) = (props.look, props.car);
        let body = document.borrow().body();
        let page = RacerWorld::mount(
            document,
            body,
            RacerWorldProps {
                scene: Some(Rc::clone(&scene)),
                surface: Some(surface),
                lens: Some(lens.read_only()),
                ..props
            },
        );
        Self {
            look,
            lens,
            car,
            log,
            #[cfg(test)]
            scene,
            _page: page,
        }
    }

    /// Shows `race` through `camera` from this frame: the camera, its
    /// lens and the car each written when it changed.
    ///
    /// # Errors
    /// A camera or race the scene refuses (not finite).
    pub(crate) fn show(&self, race: &Race, camera: &Camera) -> Result<(), String> {
        let refused =
            |error: vapors_scene3d::SceneError| format!("the racer's world refused: {error}");
        self.look
            .set_if_changed(camera_transform(camera, race).map_err(refused)?);
        self.lens.set_if_changed(lens(camera).map_err(refused)?);
        self.car
            .set_if_changed(car_transform(race).map_err(refused)?);
        Ok(())
    }

    /// The scene renderer's log, which outlives it: its renders, what it
    /// made and freed, and whether it was dropped.
    #[cfg(test)]
    pub(crate) fn log(&self) -> Rc<RendererLog> {
        Rc::clone(&self.log)
    }

    /// How many times the world has rendered.
    #[cfg(test)]
    pub(crate) fn renders(&self) -> u64 {
        self.log.renders()
    }

    /// The scene the world's components build.
    #[cfg(test)]
    pub(crate) fn scene(&self) -> &Rc<RefCell<Scene>> {
        &self.scene
    }
}

/// The world component's props but its scene, surface and lens: the parts'
/// geometry and the ground given to `meshes`, the camera and car placed.
fn props(meshes: &RefCell<Meshes>, (race, camera): (&Race, &Camera)) -> RacerWorldProps {
    let mut held = meshes.borrow_mut();
    let ground_material = held
        .material(crate::ground_material())
        .expect("the ground reads only the contract");
    let ground = world::ground_geometry(&mut held).expect("the ground plane");
    let parts = world::part_geometry(&mut held).expect("the racer's parts");
    RacerWorldProps {
        eye: Some(CameraHandle::new()),
        look: Signal::new(camera_transform(camera, race).expect("the racer's camera")),
        world: Signal::new(world::racers_world()),
        car: Signal::new(car_transform(race).expect("the race")),
        ground: Some(ground),
        ground_material: Some(ground_material),
        parts,
        ..RacerWorldProps::default()
    }
}
