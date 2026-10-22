//! Work that ends by sending a [`Message`] into the event loop: GPU setup
//! now, the file picker later. Native blocks on the future where it is;
//! the web spawns it on the browser's loop, since nothing may block there.

use std::sync::Arc;

use winit::event_loop::EventLoopProxy;
use winit::window::Window;

use crate::surface::Gpu;
use crate::window::Message;

/// Set up the GPU for `window` and send the result as [`Message::GpuReady`].
#[cfg(not(target_family = "wasm"))]
#[allow(clippy::needless_pass_by_value)] // same signature as the web version, which moves both
pub fn start_gpu(window: Arc<Window>, proxy: EventLoopProxy<Message>) {
    let result = pollster::block_on(Gpu::for_window(&window));
    send(&proxy, Message::GpuReady(result));
}

/// Set up the GPU for `window` and send the result as [`Message::GpuReady`].
#[cfg(target_family = "wasm")]
pub fn start_gpu(window: Arc<Window>, proxy: EventLoopProxy<Message>) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = Gpu::for_window(&window).await;
        send(&proxy, Message::GpuReady(result));
    });
}

/// Deliver `message`. A closed loop cannot be told anything, and the
/// window it belonged to is gone, so that case is dropped on purpose.
pub fn send(proxy: &EventLoopProxy<Message>, message: Message) {
    drop(proxy.send_event(message));
}
