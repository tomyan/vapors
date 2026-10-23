//! The frame's surface texture, or why there is none this redraw.

use winit::window::Window;

use crate::surface::Gpu;

/// The next surface texture, or the name of the outcome that gave none,
/// after reconfiguring where that helps and asking for another redraw.
pub fn acquire(window: &Window, gpu: &mut Gpu) -> Result<wgpu::SurfaceTexture, &'static str> {
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
