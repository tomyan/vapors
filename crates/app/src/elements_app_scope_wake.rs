//! The loop's waker for the painter's GPU error scopes
//! (`ScenePainter::wake_with`): a scope whose result arrives after its
//! frame, as on the web, wakes the loop for a frame that checks it,
//! instead of the app asking for frames until it resolves.

use std::sync::Arc;
use std::task::Wake;

use super::super::surface_handle::Run;
use super::AppSurfaces;
use crate::painter::ScenePainter;

/// Wakes the app's loop, as a [`std::task::Waker`].
struct LoopWake(Wrapped);

/// Natively the platform's waker is `Send` and `Sync`, as a
/// `std::task::Waker` needs.
#[cfg(not(target_family = "wasm"))]
type Wrapped = vapors_platform::Waker;

/// On the web the platform's waker is not `Send` (its loop proxy carries
/// wgpu's web types). Sound: wasm32-unknown-unknown has one thread, so
/// the wrapped waker is never touched off the thread that made it, and
/// `SendWrapper`'s off-thread panic cannot be reached.
#[cfg(target_family = "wasm")]
type Wrapped = send_wrapper::SendWrapper<vapors_platform::Waker>;

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
compile_error!(
    "a threaded wasm build can touch the scope waker's SendWrapper off its thread, which panics at run time; replace it (elements_app_scope_wake.rs) before building with atomics"
);

#[cfg(not(target_family = "wasm"))]
fn wrapped(waker: vapors_platform::Waker) -> Wrapped {
    waker
}

#[cfg(target_family = "wasm")]
fn wrapped(waker: vapors_platform::Waker) -> Wrapped {
    send_wrapper::SendWrapper::new(waker)
}

impl Wake for LoopWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // LoopClosed: the app failed or was dropped, and its surfaces
        // with it, so no frame is owed to the scope; the failure itself
        // was reported where it happened. Retired if the scope's watch
        // ever holds the platform waker itself rather than a std Waker,
        // whose wake has no result: it could then return LoopClosed.
        let _ = self.0.wake();
    }
}

impl AppSurfaces {
    /// Gives `painter` the loop's waker once the loop runs, if it has
    /// none: a painter made again later is given it too.
    pub(in super::super) fn give_waker(&self, painter: Option<&mut ScenePainter>) {
        let Some(painter) = painter.filter(|painter| !painter.has_waker()) else {
            return;
        };
        if let Run::Running(waker) = &self.shared.borrow().run {
            painter.wake_with(Arc::new(LoopWake(wrapped(waker.clone()))).into());
        }
    }
}

/// Gives each of `painters`, in turn, the loop's waker from one app's
/// surfaces, as when the app makes its painter again. Called from
/// `gpu_surface_backing_tests`.
#[cfg(test)]
pub(crate) fn give_one_loop_waker_to(painters: [&mut ScenePainter; 2]) {
    let surfaces = AppSurfaces::default();
    surfaces.keep_waker(&vapors_platform::Platform::detached());
    for painter in painters {
        surfaces.give_waker(Some(painter));
    }
}
