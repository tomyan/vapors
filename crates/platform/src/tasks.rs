//! Work that ends by sending a [`Message`] into the event loop: GPU setup
//! here, the file picker in `picker.rs`. Native blocks on the future where it is;
//! the web spawns it on the browser's loop, since nothing may block there.

use std::sync::Arc;

use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::gpu_watch::GpuWatch;
use crate::surface::Gpu;
use crate::window::Message;

/// Set up the GPU for `window` and send the result as [`Message::GpuReady`].
#[cfg(not(target_family = "wasm"))]
#[allow(clippy::needless_pass_by_value)] // same signature as the web version, which moves them
pub fn start_gpu(window: Arc<Window>, proxy: EventLoopProxy<Message>, watch: GpuWatch) {
    let result = pollster::block_on(Gpu::for_window(&window, &watch, &proxy));
    send(&proxy, Message::GpuReady(result));
}

/// Set up the GPU for `window` and send the result as [`Message::GpuReady`].
#[cfg(target_family = "wasm")]
pub fn start_gpu(window: Arc<Window>, proxy: EventLoopProxy<Message>, watch: GpuWatch) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = Gpu::for_window(&window, &watch, &proxy).await;
        send(&proxy, Message::GpuReady(result));
    });
}

/// What wgpu's device-lost and uncaptured-error callbacks call to wake
/// the loop with [`Message::GpuStopped`]. Natively a callback may run on
/// another thread, and the proxy goes with it.
#[cfg(not(target_family = "wasm"))]
pub fn stop_waker(proxy: EventLoopProxy<Message>) -> impl Fn() + Send + 'static {
    move || send(&proxy, Message::GpuStopped)
}

#[cfg(target_family = "wasm")]
thread_local! {
    /// The page's one thread keeps the proxy for the GPU callbacks, which
    /// wgpu requires to be `Send` though nothing on the web moves threads.
    static STOP_PROXY: std::cell::RefCell<Option<EventLoopProxy<Message>>> = const { std::cell::RefCell::new(None) };
}

/// What wgpu's device-lost and uncaptured-error callbacks call to wake
/// the loop with [`Message::GpuStopped`]: on the web, through the
/// thread's proxy.
#[cfg(target_family = "wasm")]
pub fn stop_waker(proxy: EventLoopProxy<Message>) -> impl Fn() + Send + 'static {
    STOP_PROXY.with(|slot| *slot.borrow_mut() = Some(proxy));
    || {
        STOP_PROXY.with(|slot| match slot.borrow().as_ref() {
            Some(proxy) => send(proxy, Message::GpuStopped),
            // Set above before a callback can run; if it ever is not,
            // say so: the failure still stops the loop at the next event.
            None => crate::report::report_error(
                "the GPU device was lost or a GPU call failed, and the event loop could not be woken (no proxy); it stops at the next event",
            ),
        });
    }
}

/// Deliver `message`. A closed loop cannot be told anything, and the
/// window it belonged to is gone, so that case is dropped on purpose.
pub fn send(proxy: &EventLoopProxy<Message>, message: Message) {
    drop(proxy.send_event(message));
}
