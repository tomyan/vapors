//! `?demo=wake`: the waker on the web. The app returns `Next::Wait`, so
//! it draws only when woken; the page's script wakes it through the
//! exported `vapors_demo_wake`, as an app's own script would after work
//! that started outside an event, and asks for a timed frame through
//! `vapors_demo_wait_until`. `scripts/web-wake-frames.sh` counts its
//! frames: none idle, one per wake, one at the time asked for.

use std::cell::{Cell, RefCell};

use vapors_platform::{App, Event, Frame, Instant, Next, Platform, Waker};
use vapors_scene::Color;

thread_local! {
    /// The waker, kept from the app's first event.
    static WAKER: RefCell<Option<Waker>> = const { RefCell::new(None) };
    /// The time the next frame returns as `Next::WaitUntil`, once.
    static UNTIL: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Clears to one colour; draws only when woken or the window needs it.
pub struct WakeApp;

impl App for WakeApp {
    fn event(&mut self, _event: Event, platform: &Platform) {
        WAKER.with(|w| {
            w.borrow_mut().get_or_insert_with(|| platform.waker());
        });
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let color = Color::from_rgba8(12, 34, 200, 255);
        vapors_renderer::clear_view(frame.device, frame.queue, &view, color);
        UNTIL.take().map_or(Next::Wait, Next::WaitUntil)
    }
}

/// Wake the demo once. The waker is cloned out of the cell first, so
/// the cell is not borrowed while the wake runs.
///
/// # Errors
///
/// No event has reached the app yet, so it has no waker; or the loop
/// has ended (`LoopClosed`).
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn vapors_demo_wake() -> Result<(), String> {
    let waker = WAKER.with(|w| w.borrow().clone()).ok_or(
        "the wake demo has had no event yet, so it has no waker; fix: wait for its first frame",
    )?;
    waker.wake().map_err(|closed| closed.to_string())
}

/// Make the demo's next frame return `Next::WaitUntil` this many
/// milliseconds from now; the page wakes it after.
#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn vapors_demo_wait_until(milliseconds: u32) {
    let at = Instant::now() + std::time::Duration::from_millis(milliseconds.into());
    UNTIL.set(Some(at));
}
