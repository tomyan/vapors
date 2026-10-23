//! The winit application handler: opens the window, waits for the GPU,
//! translates input, redraws.

use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use crate::PlatformError;
use crate::device_lost::stop_for;
use crate::event::Event;
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
    /// Redraws in a row that produced no texture; see [`acquire`].
    missed: u32,
    error: Option<PlatformError>,
    /// Set when a lost device stopped the loop; see `stop_if_lost`.
    stopped_for_loss: bool,
}

/// Consecutive redraws without a texture before giving up: a second at
/// 60 Hz. An occluded window on some compositors reports Occluded for
/// as long as it is hidden, so this is a first number to be measured
/// against that; the failure names the outcome so a reader can tell.
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
            stopped_for_loss: false,
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
            self.platform.lost_state(),
        );
        self.window = Some(window);
        Ok(())
    }

    fn gpu_ready(&mut self, gpu: Gpu) {
        let limits = gpu.limits();
        crate::report::report_status(&format!(
            "adapter: {}; max texture {} px, max buffer {} MiB",
            gpu.adapter_summary(),
            limits.max_texture_dimension_2d,
            limits.max_buffer_size >> 20
        ));
        self.gpu = Some(gpu);
        if let Some(window) = &self.window {
            let size = window.inner_size();
            self.app.event(
                Event::Resized {
                    width: size.width,
                    height: size.height,
                },
                &self.platform,
            );
            window.request_redraw();
        }
    }

    /// A lost device stops the loop before the next frame: no GPU call is
    /// made on it again, and the loss is reported with the fix.
    fn lost_stop(&self) -> Option<PlatformError> {
        let target = crate::device_lost::Target::this_build();
        stop_for(
            self.platform.device_lost(),
            target,
            crate::report::app_name(),
        )
    }

    /// Asks the window for a frame if the app asked for one while it
    /// handled the events just delivered.
    fn redraw_if_asked(&self) {
        if self.platform.take_redraw_request()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        crate::report::fatal(&error);
        self.error = Some(error);
        event_loop.exit();
    }

    /// Stop the loop with the loss's error, if the device is lost: the
    /// GPU is dropped so nothing calls it again, and the loss is
    /// reported once however many paths see it. True when stopped.
    fn stop_if_lost(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if self.stopped_for_loss {
            return true;
        }
        let Some(error) = self.lost_stop() else {
            return false;
        };
        self.stopped_for_loss = true;
        self.gpu = None;
        self.fail(event_loop, error);
        true
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if self.stop_if_lost(event_loop) {
            return;
        }
        let (Some(window), Some(gpu)) = (&self.window, self.gpu.as_mut()) else {
            return;
        };
        let surface_texture = match acquire(window, gpu) {
            Ok(texture) => texture,
            Err(outcome) => {
                self.missed += 1;
                if self.missed >= MISSED_LIMIT {
                    let error = PlatformError::Surface(format!(
                        "no frame for {MISSED_LIMIT} redraws in a row; the last outcome was {outcome}"
                    ));
                    self.fail(event_loop, error);
                }
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
        };
        let next = self.app.frame(&frame);
        gpu.queue.present(surface_texture);
        self.presented += 1;
        match next {
            Next::Continue => window.request_redraw(),
            Next::Wait => {}
            Next::Exit => event_loop.exit(),
        }
    }
}

#[path = "handler_events.rs"]
mod events;
