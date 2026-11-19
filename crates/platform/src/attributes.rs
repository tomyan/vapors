//! What to ask winit for: a titled, sized window on native; the page's
//! canvas on the web, which the page's CSS sizes.

use winit::window::{Window, WindowAttributes};

use crate::PlatformError;
use crate::session::{Session, refuse_unhonoured};
use crate::window::WindowOptions;

#[cfg(not(target_family = "wasm"))]
pub fn for_options(
    options: &WindowOptions,
    session: Session,
) -> Result<WindowAttributes, PlatformError> {
    use winit::dpi::LogicalSize;

    refuse_unhonoured(options, session)?;
    let attributes = Window::default_attributes()
        .with_title(&options.title)
        .with_decorations(options.decorations)
        .with_inner_size(LogicalSize::new(options.width, options.height));
    Ok(with_app_id(attributes, options))
}

/// Winit's general name and instance for the app id; refused earlier
/// unless the session honours it.
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn with_app_id(attributes: WindowAttributes, options: &WindowOptions) -> WindowAttributes {
    use winit::platform::wayland::WindowAttributesExtWayland;

    match &options.app_id {
        Some(id) => attributes.with_name(id, id),
        None => attributes,
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd",
    target_family = "wasm"
)))]
fn with_app_id(attributes: WindowAttributes, _options: &WindowOptions) -> WindowAttributes {
    attributes
}

#[cfg(target_family = "wasm")]
pub fn for_options(
    options: &WindowOptions,
    session: Session,
) -> Result<WindowAttributes, PlatformError> {
    use winit::platform::web::WindowAttributesExtWebSys;

    refuse_unhonoured(options, session)?;
    let canvas = crate::web::canvas()?;
    Ok(Window::default_attributes()
        .with_title(&options.title)
        // winit focuses the canvas when it makes the window, whoever
        // holds focus; `canvas_focus_web` does it unless the page's own
        // element has it
        .with_active(false)
        .with_canvas(Some(canvas)))
}
