//! Work that ends by sending a [`Message`] into the event loop: GPU setup
//! here, the file picker in `picker.rs`. Native blocks on the future where it is;
//! the web spawns it on the browser's loop, since nothing may block there.

use std::sync::Arc;

use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::device_lost::DeviceLost;
use crate::surface::Gpu;
use crate::window::Message;

/// Set up the GPU for `window` and send the result as [`Message::GpuReady`].
#[cfg(not(target_family = "wasm"))]
#[allow(clippy::needless_pass_by_value)] // same signature as the web version, which moves them
pub fn start_gpu(window: Arc<Window>, proxy: EventLoopProxy<Message>, lost: DeviceLost) {
    let result = pollster::block_on(Gpu::for_window(&window, &lost, proxy.clone()));
    send(&proxy, Message::GpuReady(result));
}

/// Set up the GPU for `window` and send the result as [`Message::GpuReady`].
#[cfg(target_family = "wasm")]
pub fn start_gpu(window: Arc<Window>, proxy: EventLoopProxy<Message>, lost: DeviceLost) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = Gpu::for_window(&window, &lost, proxy.clone()).await;
        send(&proxy, Message::GpuReady(result));
    });
}

/// What wgpu's device-lost callback calls to wake the loop with
/// [`Message::DeviceLost`]. Natively the callback may run on another
/// thread, and the proxy goes with it.
#[cfg(not(target_family = "wasm"))]
pub fn lost_waker(proxy: EventLoopProxy<Message>) -> impl Fn() + Send + 'static {
    move || send(&proxy, Message::DeviceLost)
}

#[cfg(target_family = "wasm")]
thread_local! {
    /// The page's one thread keeps the proxy for the lost callback, which
    /// wgpu requires to be `Send` though nothing on the web moves threads.
    static LOST_PROXY: std::cell::RefCell<Option<EventLoopProxy<Message>>> = const { std::cell::RefCell::new(None) };
}

/// What wgpu's device-lost callback calls to wake the loop with
/// [`Message::DeviceLost`]: on the web, through the thread's proxy.
#[cfg(target_family = "wasm")]
pub fn lost_waker(proxy: EventLoopProxy<Message>) -> impl Fn() + Send + 'static {
    LOST_PROXY.with(|slot| *slot.borrow_mut() = Some(proxy));
    || {
        LOST_PROXY.with(|slot| match slot.borrow().as_ref() {
            Some(proxy) => send(proxy, Message::DeviceLost),
            // Set above before the callback can run; if it ever is not,
            // say so: the loss still stops the loop at the next frame.
            None => crate::report::report_error(
                "the GPU device was lost and the event loop could not be woken (no proxy); it stops at the next frame",
            ),
        });
    }
}

/// Deliver `message`. A closed loop cannot be told anything, and the
/// window it belonged to is gone, so that case is dropped on purpose.
pub fn send(proxy: &EventLoopProxy<Message>, message: Message) {
    drop(proxy.send_event(message));
}
