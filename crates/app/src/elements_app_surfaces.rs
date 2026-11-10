//! Surface renderers on an [`ElementsApp`] (docs/gpu-surface.md). The
//! app mounts them before its painter exists, which is made on the
//! first frame: the renderers wait in `pending` and go to the painter
//! when it is made. A taken name is refused at `mount`. `mount` hands
//! back a [`SurfaceHandle`] (`elements_app_surface_handle.rs`); the app
//! tells its handles when its loop starts and ends.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_platform::Platform;

use super::ElementsApp;
use super::surface_handle::{Run, Shared, SurfaceHandle};
use crate::painter::{MountError, ScenePainter};
use crate::surface::SurfaceRenderer;

/// The app's renderers before the painter exists, and what its handles
/// share with it.
#[derive(Default)]
pub(super) struct AppSurfaces {
    pending: Vec<(String, Box<dyn SurfaceRenderer>)>,
    shared: Rc<RefCell<Shared>>,
}

impl ElementsApp {
    /// Mounts `renderer` for the `gpu-surface` named `name`, before or
    /// after the first frame; keep the handle to invalidate it.
    ///
    /// # Errors
    ///
    /// [`MountError::AlreadyMounted`] when a renderer has that name.
    pub fn mount(
        &mut self,
        name: &str,
        renderer: Box<dyn SurfaceRenderer>,
    ) -> Result<SurfaceHandle, MountError> {
        self.surfaces.claim(name)?;
        match &mut self.painter {
            Some(painter) => painter
                .mount(name, renderer)
                .inspect_err(|_| self.surfaces.release(name))?,
            None => self.surfaces.pending.push((name.to_owned(), renderer)),
        }
        Ok(self.surfaces.handle(name))
    }

    /// Unmounts the renderer named `name` and hands it back; dropping it
    /// is its teardown. Its handles are refused from then on.
    ///
    /// # Errors
    ///
    /// [`MountError::NotMounted`] when no renderer has that name.
    pub fn unmount(&mut self, name: &str) -> Result<Box<dyn SurfaceRenderer>, MountError> {
        let mut shared = self.surfaces.shared.borrow_mut();
        if !shared.mounted.remove(name) {
            return Err(MountError::NotMounted(name.to_owned()));
        }
        shared.stale.remove(name);
        drop(shared);
        match &mut self.painter {
            Some(painter) => painter.unmount(name),
            None => Ok(self.surfaces.take_pending(name)),
        }
    }
}

impl AppSurfaces {
    fn handle(&self, name: &str) -> SurfaceHandle {
        SurfaceHandle {
            name: name.to_owned(),
            shared: Rc::clone(&self.shared),
        }
    }

    fn claim(&self, name: &str) -> Result<(), MountError> {
        let mut shared = self.shared.borrow_mut();
        if shared.mounted.insert(name.to_owned()) {
            Ok(())
        } else {
            Err(MountError::AlreadyMounted(name.to_owned()))
        }
    }

    fn release(&self, name: &str) {
        self.shared.borrow_mut().mounted.remove(name);
    }

    fn take_pending(&mut self, name: &str) -> Box<dyn SurfaceRenderer> {
        let at = self.pending.iter().position(|(pending, _)| pending == name);
        let at = at.expect("a mounted name with no painter is pending");
        self.pending.remove(at).1
    }

    /// Keeps the platform's waker, once: the loop has started. A closed
    /// loop stays closed.
    pub(super) fn keep_waker(&self, platform: &Platform) {
        let mut shared = self.shared.borrow_mut();
        if matches!(shared.run, Run::NotStarted) {
            shared.run = Run::Running(platform.waker());
        }
    }

    /// The loop has ended: the app failed or is dropped. Handles refuse
    /// from then on.
    pub(super) fn close(&self) {
        self.shared.borrow_mut().run = Run::Closed;
    }

    /// True when a surface was invalidated since the last frame.
    pub(super) fn any_stale(&self) -> bool {
        !self.shared.borrow().stale.is_empty()
    }

    /// Mounts the pending renderers on the painter just made, and passes
    /// it the surfaces invalidated since the last frame.
    ///
    /// # Errors
    ///
    /// When the painter disagrees with the app about what is mounted.
    pub(super) fn bring_to(&mut self, painter: &mut ScenePainter) -> Result<(), MountError> {
        for (name, renderer) in self.pending.drain(..) {
            painter.mount(&name, renderer)?;
        }
        let stale = std::mem::take(&mut self.shared.borrow_mut().stale);
        stale.iter().try_for_each(|name| painter.invalidate(name))
    }
}

impl Drop for AppSurfaces {
    fn drop(&mut self) {
        self.close();
    }
}
