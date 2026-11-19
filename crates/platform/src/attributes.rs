//! What to ask winit for: a titled, sized window on native; the page's
//! canvas on the web, which the page's CSS sizes.

use winit::window::{Window, WindowAttributes};

use crate::PlatformError;
use crate::window::WindowOptions;

#[cfg(not(target_family = "wasm"))]
#[allow(clippy::unnecessary_wraps)] // same signature as the web version, which can fail
pub fn for_options(options: &WindowOptions) -> Result<WindowAttributes, PlatformError> {
    use winit::dpi::LogicalSize;

    let attributes = Window::default_attributes()
        .with_title(&options.title)
        .with_decorations(options.decorations)
        .with_inner_size(LogicalSize::new(options.width, options.height));
    Ok(with_app_id(attributes, options))
}

/// The (general, instance) names winit takes for the Wayland app id and
/// the X11 class: both are the app id.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn class_names(options: &WindowOptions) -> Option<(String, String)> {
    options.app_id.clone().map(|id| (id.clone(), id))
}

#[cfg(target_os = "linux")]
fn with_app_id(attributes: WindowAttributes, options: &WindowOptions) -> WindowAttributes {
    use winit::platform::wayland::WindowAttributesExtWayland;

    match class_names(options) {
        Some((general, instance)) => attributes.with_name(general, instance),
        None => attributes,
    }
}

#[cfg(all(not(target_os = "linux"), not(target_family = "wasm")))]
fn with_app_id(attributes: WindowAttributes, _options: &WindowOptions) -> WindowAttributes {
    attributes
}

#[cfg(target_family = "wasm")]
pub fn for_options(options: &WindowOptions) -> Result<WindowAttributes, PlatformError> {
    use winit::platform::web::WindowAttributesExtWebSys;

    let canvas = crate::web::canvas()?;
    Ok(Window::default_attributes()
        .with_title(&options.title)
        // winit focuses the canvas when it makes the window, whoever
        // holds focus; `canvas_focus_web` does it unless the page's own
        // element has it
        .with_active(false)
        .with_canvas(Some(canvas)))
}
