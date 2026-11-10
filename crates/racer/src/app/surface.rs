//! The track's `gpu-surface`: the scene the renderer draws, and when the
//! surface animates. It animates while the race runs or the camera
//! moves; otherwise a changed scene renders once and the app waits.
use crate::gpu::{Scene, Track};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vapors_app::{ElementsApp, InvalidateError, SurfaceHandle};

/// The `name` of the HUD's `gpu-surface`.
const NAME: &str = "track";

pub(crate) struct TrackSurface {
    handle: SurfaceHandle,
    scene: Rc<RefCell<Scene>>,
    // RELAXED: read only by the surface tests until the IU5 counters
    // (docs/plans/3d-1c.md, PR 3) report it per frame.
    #[cfg_attr(not(test), allow(dead_code))]
    renders: Rc<Cell<u64>>,
    animating: bool,
}
impl TrackSurface {
    /// Mounts the track's renderer on `app`, which has no other.
    pub(crate) fn mount(app: &mut ElementsApp) -> Self {
        let scene = Rc::new(RefCell::new(Scene::default()));
        let renders = Rc::new(Cell::new(0));
        let track = Box::new(Track::new(Rc::clone(&scene), Rc::clone(&renders)));
        let handle = app.mount(NAME, track).expect("a new app has no renderers");
        Self {
            handle,
            scene,
            renders,
            animating: false,
        }
    }
    /// Shows `scene` from the next render: every frame while `moving`,
    /// else once if it changed.
    pub(crate) fn show(&mut self, scene: Scene, moving: bool) -> Result<(), InvalidateError> {
        let changed = *self.scene.borrow() != scene;
        if changed {
            *self.scene.borrow_mut() = scene;
        }
        self.animate(moving)?;
        if changed && !moving {
            self.handle.invalidate()?;
        }
        Ok(())
    }
    /// How many times the track has rendered.
    #[cfg(test)]
    pub(crate) fn renders(&self) -> u64 {
        self.renders.get()
    }
    fn animate(&mut self, moving: bool) -> Result<(), InvalidateError> {
        if moving && !self.animating {
            self.handle.request_animation()?;
        } else if !moving && self.animating {
            self.handle.stop_animation()?;
        }
        self.animating = moving;
        Ok(())
    }
}
