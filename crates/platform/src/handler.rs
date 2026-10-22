//! The winit application handler: opens the window, waits for the GPU,
//! translates input, redraws.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

use crate::PlatformError;
use crate::event::Event;
use crate::input::Input;
use crate::surface::Gpu;
use crate::tasks;
use crate::window::{App, Frame, Message, Next, WindowOptions};

pub struct Handler {
    options: WindowOptions,
    app: Box<dyn App>,
    proxy: EventLoopProxy<Message>,
    input: Input,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    presented: u64,
    error: Option<PlatformError>,
}

impl Handler {
    pub fn new(options: WindowOptions, app: Box<dyn App>, proxy: EventLoopProxy<Message>) -> Self {
        Self {
            options,
            app,
            proxy,
            input: Input::new(),
            window: None,
            gpu: None,
            presented: 0,
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
        tasks::start_gpu(Arc::clone(&window), self.proxy.clone());
        self.window = Some(window);
        Ok(())
    }

    fn gpu_ready(&mut self, gpu: Gpu) {
        crate::report::status(&format!("adapter: {}", gpu.adapter_summary()));
        self.gpu = Some(gpu);
        if let Some(window) = &self.window {
            let size = window.inner_size();
            self.app.event(Event::Resized {
                width: size.width,
                height: size.height,
            });
            window.request_redraw();
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        crate::report::error(&error);
        self.error = Some(error);
        event_loop.exit();
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(gpu)) = (&self.window, self.gpu.as_mut()) else {
            return;
        };
        let Some(surface_texture) = acquire(window, gpu) else {
            return;
        };
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

/// The next surface texture, or `None` after reconfiguring and asking for
/// another redraw.
fn acquire(window: &Window, gpu: &mut Gpu) -> Option<wgpu::SurfaceTexture> {
    match gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
            Some(t)
        }
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            window.request_redraw();
            None
        }
        _ => {
            let size = window.inner_size();
            gpu.resize(size.width, size.height);
            window.request_redraw();
            None
        }
    }
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
            self.app.event(app_event);
        }
        if let WindowEvent::RedrawRequested = event {
            self.redraw(event_loop);
        }
    }
}
