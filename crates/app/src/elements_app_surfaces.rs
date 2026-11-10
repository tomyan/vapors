//! Surface renderers on an [`ElementsApp`] (docs/gpu-surface.md). The
//! app mounts them before its painter exists, which is made on the
//! first frame: the renderers wait in `pending` and go to the painter
//! when it is made. A taken name is refused at `mount`. `mount` hands
//! back a [`SurfaceHandle`] whose `invalidate` works while `run` owns
//! the app: it marks the surface stale, for the next frame to pass to
//! the painter, and wakes the loop with the waker the app kept from the
//! platform. Handles are single-threaded, as the document is.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;

use vapors_platform::{Platform, Waker};

use super::ElementsApp;
use crate::painter::{MountError, ScenePainter};
use crate::surface::SurfaceRenderer;

/// The app's renderers before the painter exists, and what its handles
/// share with it.
#[derive(Default)]
pub(super) struct AppSurfaces {
    pending: Vec<(String, Box<dyn SurfaceRenderer>)>,
    shared: Rc<RefCell<Shared>>,
}

/// What the app and its handles share.
#[derive(Default)]
struct Shared {
    /// Every mounted name, pending or on the painter.
    mounted: BTreeSet<String>,
    /// Names invalidated since the last frame passed them on.
    stale: BTreeSet<String>,
    /// The platform's waker, kept from the first event or frame.
    waker: Option<Waker>,
}

/// Invalidates one mounted surface from outside the app; cheap to clone.
#[derive(Clone)]
pub struct SurfaceHandle {
    name: String,
    shared: Rc<RefCell<Shared>>,
}

/// Why [`SurfaceHandle::invalidate`] did nothing.
#[derive(Debug, PartialEq, Eq)]
pub enum InvalidateError {
    /// The surface's renderer was unmounted.
    NotMounted(String),
    /// The event loop has ended, so no frame will render the surface.
    LoopClosed(String),
}

impl fmt::Display for InvalidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotMounted(name) => write!(
                f,
                "the gpu-surface {name:?} has no renderer mounted, so it was not invalidated; mount one with ElementsApp::mount and invalidate through its handle"
            ),
            Self::LoopClosed(name) => write!(
                f,
                "the event loop has ended, so the gpu-surface {name:?} will not render again; invalidate only while the app runs"
            ),
        }
    }
}

impl std::error::Error for InvalidateError {}

impl SurfaceHandle {
    /// The surface's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Renders the surface on the next frame that paints it, and wakes
    /// the loop for that frame. Before the app's first event there is
    /// no waker yet; the first frame comes without one.
    ///
    /// # Errors
    ///
    /// [`InvalidateError::NotMounted`] when the renderer was unmounted;
    /// [`InvalidateError::LoopClosed`] when the loop has ended.
    pub fn invalidate(&self) -> Result<(), InvalidateError> {
        let mut shared = self.shared.borrow_mut();
        if !shared.mounted.contains(&self.name) {
            return Err(InvalidateError::NotMounted(self.name.clone()));
        }
        shared.stale.insert(self.name.clone());
        let waker = shared.waker.clone();
        drop(shared);
        match waker.map(|waker| waker.wake()) {
            Some(Err(_)) => Err(InvalidateError::LoopClosed(self.name.clone())),
            _ => Ok(()),
        }
    }
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
        if !self
            .surfaces
            .shared
            .borrow_mut()
            .mounted
            .insert(name.to_owned())
        {
            return Err(MountError::AlreadyMounted(name.to_owned()));
        }
        match &mut self.painter {
            Some(painter) => painter.mount(name, renderer).inspect_err(|_| {
                self.surfaces.shared.borrow_mut().mounted.remove(name);
            })?,
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

    fn take_pending(&mut self, name: &str) -> Box<dyn SurfaceRenderer> {
        let at = self.pending.iter().position(|(pending, _)| pending == name);
        let at = at.expect("a mounted name with no painter is pending");
        self.pending.remove(at).1
    }

    /// Keeps the platform's waker, once.
    pub(super) fn keep_waker(&self, platform: &Platform) {
        let mut shared = self.shared.borrow_mut();
        shared.waker.get_or_insert_with(|| platform.waker());
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
