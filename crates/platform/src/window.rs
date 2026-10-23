//! The event loop: one window, redrawn continuously, each redraw handed
//! to the app as a [`Frame`], each input handed to it as an [`Event`].
//!
//! On native, [`run`] drives the loop to the end. On the web the loop is
//! the browser's, so [`run`] attaches to the page's canvas, spawns the
//! handler and returns at once; failures after that are reported on the
//! page and in the console.

use winit::event_loop::EventLoop;

use crate::PlatformError;
use crate::event::Event;
use crate::handler::Handler;
use crate::platform::Platform;
use crate::surface::Gpu;

/// How to open the window. On the web the page's CSS sizes the canvas and
/// `width` and `height` are not used.
pub struct WindowOptions {
    /// The name that prefixes every reported line, as `touche:`. When
    /// empty, the title's first word (without a trailing colon) is used.
    pub app_name: String,
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
    /// Device pixels per logical pixel: the window's scale factor, the
    /// page's `devicePixelRatio` on the web. 2.0 on a high-density display.
    pub scale_factor: f64,
}

impl Frame<'_> {
    /// The limits the window's device was opened with: the defaults, with
    /// `max_buffer_size`, `max_texture_dimension_2d` and
    /// `max_storage_buffer_binding_size` raised to the adapter's.
    #[must_use]
    pub fn limits(&self) -> wgpu::Limits {
        self.device.limits()
    }
}

/// What the app wants after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Draw again as soon as possible: every display refresh, for an app
    /// that is animating.
    Continue,
    /// Draw again only when something changes: when the app calls
    /// [`Platform::request_redraw`](crate::Platform::request_redraw)
    /// while handling an event, or the window needs it (shown, resized,
    /// its scale changed). An idle app that returns this draws nothing.
    Wait,
    /// Close the window and return from [`run`].
    Exit,
}

/// The app's side of the window: it gets every input and draws every frame.
pub trait App {
    /// An input arrived. Called between frames. `platform` is for what the
    /// app may ask for in return, such as a file picker.
    fn event(&mut self, event: Event, platform: &Platform);
    /// Draw this frame.
    fn frame(&mut self, frame: &Frame<'_>) -> Next;
}

/// What the platform's own background work sends into the event loop.
pub(crate) enum Message {
    /// The window's GPU is set up, or could not be.
    GpuReady(Result<Gpu, PlatformError>),
    /// The page's Open button was pressed.
    #[cfg_attr(not(target_family = "wasm"), allow(dead_code))] // only the web has the button
    OpenRequested,
    /// The user chose a file in the picker.
    FileChosen {
        /// Its name, without a directory.
        name: String,
        /// Its contents.
        bytes: Vec<u8>,
    },
    /// wgpu reported the window's device lost; the loss is in the
    /// platform's state. Sent so the loop stops now, not at the next
    /// frame, which a backgrounded tab may never draw.
    DeviceLost,
}

/// Open a window and drive `app` until it returns [`Next::Exit`] or the
/// window is closed.
///
/// # Errors
///
/// [`PlatformError`] when the event loop, window, adapter, device or
/// surface cannot be set up. Each variant's message names what was found.
#[cfg(not(target_family = "wasm"))]
pub fn run(options: WindowOptions, app: impl App + 'static) -> Result<(), PlatformError> {
    crate::report::set_prefix(&options.app_name, &options.title);
    crate::log_forward::install();
    let event_loop = EventLoop::with_user_event()
        .build()
        .map_err(|e| PlatformError::Windowing(e.to_string()))?;
    let platform = Platform::new(event_loop.create_proxy());
    let mut handler = Handler::new(options, Box::new(app), platform);
    event_loop
        .run_app(&mut handler)
        .map_err(|e| PlatformError::Windowing(e.to_string()))?;
    handler.into_result()
}

/// Attach to the page's `<canvas id="vapors">`, spawn the handler on the
/// browser's event loop and return at once. Errors here are returned for
/// the caller to report, as on native; anything that fails after this
/// returns is written to the console and to the `status` element.
///
/// # Errors
///
/// [`PlatformError::NoWebGpu`] when the browser has no `navigator.gpu`,
/// [`PlatformError::Windowing`] when the page has no such canvas or the
/// browser refuses an event loop.
#[cfg(target_family = "wasm")]
pub fn run(options: WindowOptions, app: impl App + 'static) -> Result<(), PlatformError> {
    use winit::platform::web::EventLoopExtWebSys;

    crate::report::set_prefix(&options.app_name, &options.title);
    crate::web::install_panic_hook();

    crate::webgpu::required(crate::web::has_webgpu())?;
    crate::web::canvas()?;
    let event_loop = EventLoop::with_user_event()
        .build()
        .map_err(|e| PlatformError::Windowing(e.to_string()))?;
    let platform = Platform::new(event_loop.create_proxy());
    crate::web::wire_open_button(platform.proxy());
    let handler = Handler::new(options, Box::new(app), platform);
    event_loop.spawn_app(handler);
    Ok(())
}
