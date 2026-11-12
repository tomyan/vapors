//! The track's `gpu-surface`: the scene the renderer draws, and when the
//! surface animates. It animates while the race runs or the camera
//! moves; otherwise a changed scene renders once and the app waits.
//! The frame where motion ends asks for no next frame. With a changed
//! scene it renders as an animated frame and stops the animation after
//! its paint, from the app's frame-times hook, so it needs no
//! invalidate, whose wake would draw one (docs/gpu-surface.md,
//! "Animation"); with an unchanged one it stops before the paint and
//! renders nothing.
use crate::gpu::{Scene, Track, TrackLog};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use vapors_app::{ElementsApp, FrameTimesHook, InvalidateError, SurfaceHandle};

/// The `name` of the HUD's `gpu-surface`.
const NAME: &str = "track";

pub(crate) struct TrackSurface {
    handle: SurfaceHandle,
    scene: Rc<RefCell<Scene>>,
    // RELAXED: read only by the surface tests until the IU5 counters
    // (docs/plans/3d-1c.md, PR 3) report it per frame.
    #[cfg_attr(not(test), allow(dead_code))]
    log: Rc<TrackLog>,
    animating: bool,
    /// Set by the frame where motion ends; the hook stops after its paint.
    settling: Rc<Cell<bool>>,
    /// What the hook's `stop_animation` refused, for `after_frame`.
    refused: Rc<RefCell<Option<InvalidateError>>>,
}
impl TrackSurface {
    /// Mounts the track's renderer on `app`, which has no other and no
    /// frame-times hook; hands back `app` with the hook that stops.
    pub(crate) fn mount(mut app: ElementsApp) -> (ElementsApp, Self) {
        let scene = Rc::new(RefCell::new(Scene::default()));
        let log = Rc::new(TrackLog::default());
        let track = Box::new(Track::new(Rc::clone(&scene), Rc::clone(&log)));
        let handle = app.mount(NAME, track).expect("a new app has no renderers");
        let surface = Self {
            handle,
            scene,
            log,
            animating: false,
            settling: Rc::default(),
            refused: Rc::default(),
        };
        (app.with_frame_times(surface.stop_after_paint()), surface)
    }
    /// The frame-times hook: runs after the frame's paint and before
    /// the app decides on the next frame.
    fn stop_after_paint(&self) -> FrameTimesHook {
        let (handle, settling) = (self.handle.clone(), Rc::clone(&self.settling));
        let refused = Rc::clone(&self.refused);
        Box::new(move |_| {
            if settling.take() {
                *refused.borrow_mut() = handle.stop_animation().err();
            }
        })
    }
    /// After the app's frame: what the hook's stop refused, if anything.
    pub(crate) fn after_frame(&self) -> Result<(), InvalidateError> {
        self.refused.take().map_or(Ok(()), Err)
    }
    /// Shows `scene` from this frame's render: every frame while
    /// `moving`, else once if it changed. Call it before the app's frame.
    pub(crate) fn show(&mut self, scene: Scene, moving: bool) -> Result<(), InvalidateError> {
        let changed = *self.scene.borrow() != scene;
        if changed {
            *self.scene.borrow_mut() = scene;
        }
        if moving && !self.animating {
            self.handle.request_animation()?;
        } else if !moving && self.animating {
            self.settle(changed)?;
        } else if !moving && changed {
            self.handle.invalidate()?;
        }
        self.animating = moving;
        Ok(())
    }
    /// How many times the track has rendered.
    #[cfg(test)]
    pub(crate) fn renders(&self) -> u64 {
        self.log.renders.get()
    }
    /// What the track's renderer has done.
    #[cfg(test)]
    pub(crate) fn log(&self) -> &TrackLog {
        &self.log
    }
    /// Motion ends this frame. A changed scene renders as the last
    /// animated frame and the hook stops after its paint; an unchanged
    /// one stops before the paint, which then renders nothing.
    fn settle(&mut self, changed: bool) -> Result<(), InvalidateError> {
        if changed {
            self.settling.set(true);
            return Ok(());
        }
        self.handle.stop_animation()
    }
}
