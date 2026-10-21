//! The event loop: one window, redrawn continuously, each redraw handed
//! to the app as a [`Frame`], each input handed to it as an [`Event`].

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::PlatformError;
use crate::event::{Event, from_winit};
use crate::surface::Gpu;

/// How to open the window.
pub struct WindowOptions {
    /// Title bar text.
    pub title: String,
    /// Logical width in points.
    pub width: u32,
    /// Logical height in points.
    pub height: u32,
}

/// One redraw: the surface texture to draw into and the device to do it with.
pub struct Frame<'a> {
    /// The adapter drawing this window, for its name in reports.
    pub adapter: &'a wgpu::Adapter,
    /// Device for this window.
    pub device: &'a wgpu::Device,
    /// Queue for this window.
    pub queue: &'a wgpu::Queue,
    /// The surface texture for this frame; drawn into, then presented.
    pub texture: &'a wgpu::Texture,
    /// Frames presented before this one. Zero on the first frame.
    pub index: u64,
}

/// What the app wants after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Keep going.
    Continue,
    /// Close the window and return from [`run`].
    Exit,
}

/// The app's side of the window: it gets every input and draws every frame.
pub trait App {
    /// An input arrived. Called between frames.
    fn event(&mut self, event: Event);
    /// Draw this frame.
    fn frame(&mut self, frame: &Frame<'_>) -> Next;
}

/// Open a window and drive `app` until it returns [`Next::Exit`] or the
/// window is closed.
///
/// # Errors
///
/// [`PlatformError`] when the event loop, window, adapter, device or
/// surface cannot be set up. Each variant's message names what was found.
pub fn run(options: WindowOptions, app: impl App + 'static) -> Result<(), PlatformError> {
    let event_loop = EventLoop::new().map_err(|e| PlatformError::Windowing(e.to_string()))?;
    let mut handler = Handler {
        options,
        app: Box::new(app),
        state: None,
        presented: 0,
        error: None,
    };
    event_loop
        .run_app(&mut handler)
        .map_err(|e| PlatformError::Windowing(e.to_string()))?;
    match handler.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct State {
    window: Arc<Window>,
    gpu: Gpu,
}

struct Handler {
    options: WindowOptions,
    app: Box<dyn App>,
    state: Option<State>,
    presented: u64,
    error: Option<PlatformError>,
}

impl Handler {
    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), PlatformError> {
        let attributes = Window::default_attributes()
            .with_title(&self.options.title)
            .with_inner_size(LogicalSize::new(self.options.width, self.options.height));
        let window = event_loop
            .create_window(attributes)
            .map_err(|e| PlatformError::Windowing(e.to_string()))?;
        let window = Arc::new(window);
        let gpu = Gpu::for_window(&window)?;
        let size = window.inner_size();
        self.app.event(Event::Resized {
            width: size.width,
            height: size.height,
        });
        window.request_redraw();
        self.state = Some(State { window, gpu });
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let Some(surface_texture) = acquire(state) else {
            return;
        };
        let frame = Frame {
            adapter: &state.gpu.adapter,
            device: &state.gpu.device,
            queue: &state.gpu.queue,
            texture: &surface_texture.texture,
            index: self.presented,
        };
        let next = self.app.frame(&frame);
        state.gpu.queue.present(surface_texture);
        self.presented += 1;
        match next {
            Next::Continue => state.window.request_redraw(),
            Next::Exit => event_loop.exit(),
        }
    }
}

/// The next surface texture, or `None` after reconfiguring and asking for
/// another redraw.
fn acquire(state: &mut State) -> Option<wgpu::SurfaceTexture> {
    match state.gpu.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
            Some(t)
        }
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            state.window.request_redraw();
            None
        }
        _ => {
            let size = state.window.inner_size();
            state.gpu.resize(size.width, size.height);
            state.window.request_redraw();
            None
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none()
            && let Err(error) = self.open(event_loop)
        {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }
        if let WindowEvent::Resized(size) = event
            && let Some(state) = self.state.as_mut()
        {
            state.gpu.resize(size.width, size.height);
        }
        if let Some(app_event) = from_winit(&event) {
            self.app.event(app_event);
        }
        if let WindowEvent::RedrawRequested = event {
            self.redraw(event_loop);
        }
    }
}
