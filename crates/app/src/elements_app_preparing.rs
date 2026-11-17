//! Preparing the next frame: while a [`Preparing`] lives, a surface's
//! invalidation marks it stale for the frame that follows and wakes no
//! loop. An app that writes its scene's signals before calling
//! `ElementsApp::frame` (the racer, one-scene R3a2) holds one around
//! those writes, so the frame that draws them is not followed by a woken
//! frame that draws nothing. The caller drops it and draws a frame right
//! after: `frame` and `event` refuse while one lives, since an
//! invalidation made under a guard kept longer would wake nothing.

use std::cell::RefCell;
use std::rc::Rc;

use super::ElementsApp;
use super::surface_handle::Shared;

/// While it lives, invalidations are for the frame being prepared.
#[must_use = "invalidations wake the loop again once it is dropped"]
pub struct Preparing {
    shared: Rc<RefCell<Shared>>,
}

impl ElementsApp {
    /// Starts preparing the next frame, until the guard is dropped:
    /// invalidations made meanwhile are drawn by the next `frame` and wake
    /// no loop (`Invalidated::ForTheNextFrame`).
    pub fn preparing(&self) -> Preparing {
        let shared = Rc::clone(&self.surfaces.shared);
        shared.borrow_mut().preparing += 1;
        Preparing { shared }
    }
}

impl ElementsApp {
    /// Refused while a [`Preparing`] lives: entering `frame` or `event`
    /// with one held means it outlives the preparation it was for.
    pub(super) fn not_preparing(&self, entered: &str) -> Result<(), String> {
        let held = self.surfaces.shared.borrow().preparing;
        if held == 0 {
            return Ok(());
        }
        Err(format!(
            "ElementsApp::{entered} was called while {held} Preparing guard(s) live: an invalidation under one wakes no loop, so a guard kept past its frame's preparation leaves a surface waiting; drop the guard before calling ElementsApp::{entered}"
        ))
    }
}

impl Drop for Preparing {
    fn drop(&mut self) {
        self.shared.borrow_mut().preparing -= 1;
    }
}
