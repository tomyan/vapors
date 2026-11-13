//! The handler's part in the GPU's life: the device's arrival after the
//! window opened, and the loop stopped on its loss.

use winit::event_loop::ActiveEventLoop;

use super::Handler;
use crate::event::Event;
use crate::surface::Gpu;
use crate::{PlatformError, Start};

impl Handler {
    pub(super) fn gpu_ready(&mut self, event_loop: &ActiveEventLoop, gpu: Gpu) {
        let limits = gpu.limits();
        crate::report::report_startup(&format!(
            "adapter: {}; max texture {} px, max buffer {} MiB",
            gpu.adapter_summary(),
            limits.max_texture_dimension_2d,
            limits.max_buffer_size >> 20
        ));
        let started = self.start_app(&gpu);
        self.gpu = Some(gpu);
        if let Err(error) = started {
            self.fail(event_loop, error);
            return;
        }
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

    /// The app's start on `gpu`, at the window's size, before the first
    /// frame; refused when the window has no size yet.
    fn start_app(&mut self, gpu: &Gpu) -> Result<(), PlatformError> {
        let window = self
            .window
            .as_ref()
            .ok_or_else(|| no_size("there is no window"))?;
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err(no_size(&format!(
                "the window is {}x{}",
                size.width, size.height
            )));
        }
        let start = Start {
            adapter: &gpu.adapter,
            device: &gpu.device,
            queue: &gpu.queue,
            format: gpu.config.format,
            width: size.width,
            height: size.height,
            scale_factor: window.scale_factor(),
        };
        self.app.start(&start).map_err(PlatformError::Start)
    }

    /// A lost device or an error no scope caught stops the loop before
    /// the next frame: no GPU call is made on it again, and the failure
    /// is reported with the fix.
    fn gpu_stop(&self) -> Option<PlatformError> {
        let target = crate::device_lost::Target::this_build();
        self.platform
            .gpu_watch()
            .stop(target, crate::report::app_name())
    }

    /// Stop the loop with the GPU watch's error, if it has one: the GPU
    /// is dropped so nothing calls it again, and the failure is reported
    /// once however many paths see it. True when stopped.
    pub(super) fn stop_if_gpu_failed(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if self.stopped_for_gpu {
            return true;
        }
        let Some(error) = self.gpu_stop() else {
            return false;
        };
        self.stopped_for_gpu = true;
        self.gpu = None;
        self.fail(event_loop, error);
        true
    }
}

/// The app cannot start: `why` the window's size is unknown.
fn no_size(why: &str) -> PlatformError {
    PlatformError::Start(format!(
        "{why} when the GPU is ready, so the first frame's size is unknown; fix: give the \
         window (on the web, the canvas) a size before the page loads the app"
    ))
}
