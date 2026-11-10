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
        // was reported where it happened.
        let _ = self.0.wake();
    }
}

impl AppSurfaces {
    /// Gives `painter` the loop's waker once the loop runs; once.
    pub(in super::super) fn give_waker(&mut self, painter: Option<&mut ScenePainter>) {
        let Some(painter) = painter.filter(|_| !self.painter_wakes) else {
            return;
        };
        if let Run::Running(waker) = &self.shared.borrow().run {
            painter.wake_with(Arc::new(LoopWake(wrapped(waker.clone()))).into());
            self.painter_wakes = true;
        }
    }
}
