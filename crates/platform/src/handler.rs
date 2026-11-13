//! The winit application handler: opens the window, waits for the GPU,
//! translates input, redraws.

use std::sync::Arc;

use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::Window;

use crate::PlatformError;
use crate::hidden_window::{MissAction, RETRY_AFTER, after_miss};
use crate::input::Input;
use crate::platform::Platform;
use crate::surface::Gpu;
use crate::surface_texture::acquire;
use crate::tasks;
use crate::window::{App, Frame, Next, WindowOptions};

pub struct Handler {
    options: WindowOptions,
    app: Box<dyn App>,
    platform: Platform,
    input: Input,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    presented: u64,
    /// Redraws in a row whose surface failed; see [`acquire`].
    missed: u32,
    error: Option<PlatformError>,
    /// Set when the GPU watch stopped the loop; see `stop_if_gpu_failed`.
    stopped_for_gpu: bool,
    /// The app's `start` has run: frames are drawn only after it.
    started: bool,
    /// The page's text input, once the window is open (the web).
    #[cfg(target_family = "wasm")]
    text_bridge: Option<crate::text_bridge_web::TextBridge>,
    /// The page's a11y mirror, once the window is open (the web).
    #[cfg(target_family = "wasm")]
    a11y: Option<a11y::WebMirror>,
    /// The system IME's composition over the focused field (native).
    #[cfg(not(target_family = "wasm"))]
    ime: std::cell::RefCell<crate::ime_state::ImeState>,
}

/// Consecutive redraws whose surface failed (`Outdated`, `Lost`,
/// `Validation`) before giving up: a second at 60 Hz, unmeasured. A
/// hidden window (`Occluded`, `Timeout`) is waited for and not counted.
/// The failure names the last outcome.
const MISSED_LIMIT: u32 = 60;

impl Handler {
    pub fn new(options: WindowOptions, app: Box<dyn App>, platform: Platform) -> Self {
        Self {
            options,
            app,
            platform,
            input: Input::new(),
            window: None,
            gpu: None,
            presented: 0,
            missed: 0,
            error: None,
            stopped_for_gpu: false,
            started: false,
            #[cfg(target_family = "wasm")]
            text_bridge: None,
            #[cfg(target_family = "wasm")]
            a11y: None,
            #[cfg(not(target_family = "wasm"))]
            ime: std::cell::RefCell::default(),
        }
    }

    /// The error recorded while running, if any.
    #[cfg(not(target_family = "wasm"))]
    pub fn into_result(self) -> Result<(), PlatformError> {
        self.error.map_or(Ok(()), Err)
    }

    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), PlatformError> {
        let attributes = crate::attributes::for_options(&self.options)?;
        let window = event_loop
            .create_window(attributes)
            .map_err(|e| PlatformError::Windowing(e.to_string()))?;
        let window = Arc::new(window);
        tasks::start_gpu(
            Arc::clone(&window),
            self.platform.proxy(),
            self.platform.gpu_watch(),
        );
        self.window = Some(window);
        #[cfg(target_family = "wasm")]
        {
            let proxy = self.platform.proxy();
            self.text_bridge = Some(crate::text_bridge_web::TextBridge::new(&proxy)?);
            self.a11y = Some(a11y::WebMirror::new()?);
        }
        Ok(())
    }

    /// Carries out what the app asked for while it handled the events
    /// just delivered or the frame just drawn: a frame, a cursor, and
    /// the text field the page's text input mirrors.
    fn redraw_if_asked(&self) {
        let Some(window) = &self.window else { return };
        if self.platform.take_redraw_request() {
            window.request_redraw();
        }
        if let Some(cursor) = self.platform.take_cursor() {
            crate::cursor::apply(window, cursor);
        }
        if let Some(field) = self.platform.take_text_field() {
            self.mirror(field.as_ref());
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        crate::report::fatal(&error);
        self.error = Some(error);
        event_loop.exit();
    }

    fn after_miss(&mut self, event_loop: &ActiveEventLoop, action: MissAction) {
        match action {
            MissAction::Wait => event_loop
                .set_control_flow(ControlFlow::WaitUntil(crate::Instant::now() + RETRY_AFTER)),
            MissAction::Count(missed) => self.missed = missed,
            MissAction::Fail(outcome) => {
                let error = PlatformError::Surface(format!(
                    "no frame for {MISSED_LIMIT} redraws in a row; the last outcome was {outcome}"
                ));
                self.fail(event_loop, error);
            }
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let now = crate::Instant::now();
        if self.stop_if_gpu_failed(event_loop) {
            return;
        }
        // no frame before the app's start (handler_gpu.rs)
        let (true, Some(window), Some(gpu)) = (self.started, &self.window, self.gpu.as_mut())
        else {
            return;
        };
        let surface_texture = match acquire(window, gpu) {
            Ok(texture) => texture,
            Err(miss) => {
                let action = after_miss(miss, self.missed, MISSED_LIMIT);
                self.after_miss(event_loop, action);
                return;
            }
        };
        self.missed = 0;
        let frame = Frame {
            adapter: &gpu.adapter,
            device: &gpu.device,
            queue: &gpu.queue,
            texture: &surface_texture.texture,
            index: self.presented,
            scale_factor: window.scale_factor(),
            now,
        };
        let next = self.app.frame(&frame);
        gpu.queue.present(surface_texture);
        self.presented += 1;
        self.app.drawn(&self.platform);
        self.redraw_if_asked();
        event_loop.set_control_flow(next.control_flow());
        match next {
            Next::Continue => window.request_redraw(),
            Next::Wait | Next::WaitUntil(_) => {}
            Next::Exit => event_loop.exit(),
        }
        // the frame's a11y update to the page's mirror; a refusal stops the loop
        if let Err(error) = self.apply_ax_updates() {
            self.fail(event_loop, error);
        }
    }
}

#[path = "handler_a11y.rs"]
mod a11y;
#[path = "handler_events.rs"]
mod events;
#[path = "handler_gpu.rs"]
mod gpu;

#[path = "handler_text.rs"]
mod text;
