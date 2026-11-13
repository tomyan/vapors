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
        self.start_when_sized(event_loop);
    }

    /// The app's start, once: when the GPU is ready and the window has a
    /// size, before the first frame (a window minimised at launch, or a
    /// web canvas not yet laid out, starts at its first non-zero
    /// resize). A refused start stops the loop.
    pub(super) fn start_when_sized(&mut self, event_loop: &ActiveEventLoop) {
        if self.started {
            return;
        }
        let (Some(window), Some(gpu)) = (&self.window, &self.gpu) else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
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
        let began = crate::Instant::now();
        let started = self.app.start(&start);
        self.started = true;
        match started {
            Ok(()) => crate::report::report_startup(&format!(
                "start (before the first frame): {:.1} ms",
                began.elapsed().as_secs_f64() * 1e3
            )),
            Err(message) => self.fail(event_loop, PlatformError::Start(message)),
        }
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
