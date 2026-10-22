//! The winit application handler: opens the window, waits for the GPU,
//! translates input, redraws.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::PlatformError;
use crate::event::Event;
use crate::input::Input;
use crate::platform::Platform;
use crate::surface::Gpu;
use crate::tasks;
use crate::window::{App, Frame, Message, Next, WindowOptions};

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
        tasks::start_gpu(Arc::clone(&window), self.platform.proxy());
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

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        crate::report::fatal(&error);
        self.error = Some(error);
        event_loop.exit();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
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
        };
        let next = self.app.frame(&frame);
        gpu.queue.present(surface_texture);
        self.presented += 1;
        match next {
            Next::Continue => window.request_redraw(),
            Next::Exit => event_loop.exit(),
        }
    }
}

/// The next surface texture, or the name of the outcome that gave none,
/// after reconfiguring where that helps and asking for another redraw.
fn acquire(window: &Window, gpu: &mut Gpu) -> Result<wgpu::SurfaceTexture, &'static str> {
    use wgpu::CurrentSurfaceTexture as Outcome;

    let outcome = match gpu.surface.get_current_texture() {
        Outcome::Success(t) | Outcome::Suboptimal(t) => return Ok(t),
        Outcome::Timeout => "Timeout",
        Outcome::Occluded => "Occluded",
        Outcome::Outdated => "Outdated",
        Outcome::Lost => "Lost",
        Outcome::Validation => "Validation",
    };
    if !matches!(outcome, "Timeout" | "Occluded") {
        let size = window.inner_size();
        gpu.resize(size.width, size.height);
    }
    window.request_redraw();
    Err(outcome)
}

impl ApplicationHandler<Message> for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.open(event_loop)
        {
            self.fail(event_loop, error);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, message: Message) {
        match message {
            Message::GpuReady(Ok(gpu)) => self.gpu_ready(gpu),
            Message::GpuReady(Err(error)) => self.fail(event_loop, error),
            Message::FileChosen { name, bytes } => self
                .app
                .event(Event::FileOpened { name, bytes }, &self.platform),
            Message::OpenRequested => self.app.event(Event::OpenRequested, &self.platform),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }
        if let WindowEvent::Resized(size) = event
            && let Some(gpu) = self.gpu.as_mut()
        {
            gpu.resize(size.width, size.height);
        }
        for app_event in self.input.translate(&event) {
            self.app.event(app_event, &self.platform);
        }
        if let WindowEvent::RedrawRequested = event {
            self.redraw(event_loop);
        }
    }
}
