//! What to ask winit for: a titled, sized window on native; the page's
//! canvas on the web, which the page's CSS sizes.

use winit::window::{Window, WindowAttributes};

use crate::PlatformError;
use crate::window::WindowOptions;

#[cfg(not(target_family = "wasm"))]
#[allow(clippy::unnecessary_wraps)] // same signature as the web version, which can fail
pub fn for_options(options: &WindowOptions) -> Result<WindowAttributes, PlatformError> {
    use winit::dpi::LogicalSize;

    Ok(Window::default_attributes()
        .with_title(&options.title)
        .with_inner_size(LogicalSize::new(options.width, options.height)))
}

#[cfg(target_family = "wasm")]
pub fn for_options(options: &WindowOptions) -> Result<WindowAttributes, PlatformError> {
    use winit::platform::web::WindowAttributesExtWebSys;

    let canvas = crate::web::canvas()?;
    Ok(Window::default_attributes()
        .with_title(&options.title)
        .with_canvas(Some(canvas)))
}
