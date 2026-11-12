//! The frame's surface texture, or why there is none this redraw.

use winit::window::Window;

use crate::surface::Gpu;

/// Why a redraw got no surface texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Miss {
    /// The window cannot be shown now: `Occluded` (minimised or covered)
    /// or `Timeout` (a Wayland compositor sends no frame callback to a
    /// hidden window, 1 s each). The loop waits for the window; this is
    /// not a failure.
    Waiting(&'static str),
    /// `Outdated`, `Lost` or `Validation`: the surface is reconfigured,
    /// and too many of these in a row stop the loop.
    Failed(&'static str),
}

/// The texture `outcome` carries, or why it carries none.
///
/// # Errors
///
/// The [`Miss`] for every outcome but `Success` and `Suboptimal`.
pub fn texture_of(outcome: wgpu::CurrentSurfaceTexture) -> Result<wgpu::SurfaceTexture, Miss> {
    use wgpu::CurrentSurfaceTexture as Outcome;

    match outcome {
        Outcome::Success(texture) | Outcome::Suboptimal(texture) => Ok(texture),
        Outcome::Timeout => Err(Miss::Waiting("Timeout")),
        Outcome::Occluded => Err(Miss::Waiting("Occluded")),
        Outcome::Outdated => Err(Miss::Failed("Outdated")),
        Outcome::Lost => Err(Miss::Failed("Lost")),
        Outcome::Validation => Err(Miss::Failed("Validation")),
    }
}

/// The next surface texture, or why there is none. A failed surface is
/// reconfigured and another redraw requested; a hidden window requests
/// none (see `hidden_window`).
pub fn acquire(window: &Window, gpu: &mut Gpu) -> Result<wgpu::SurfaceTexture, Miss> {
    let miss = match texture_of(gpu.surface.get_current_texture()) {
        Ok(texture) => return Ok(texture),
        Err(miss) => miss,
    };
    if let Miss::Failed(_) = miss {
        let size = window.inner_size();
        gpu.resize(size.width, size.height);
        window.request_redraw();
    }
    Err(miss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::CurrentSurfaceTexture as Outcome;

    #[test]
    fn a_hidden_window_is_waited_for_and_a_broken_surface_is_a_failure() {
        // Given
        let cases = [
            (Outcome::Occluded, Miss::Waiting("Occluded")),
            (Outcome::Timeout, Miss::Waiting("Timeout")),
            (Outcome::Outdated, Miss::Failed("Outdated")),
            (Outcome::Lost, Miss::Failed("Lost")),
            (Outcome::Validation, Miss::Failed("Validation")),
        ];

        for (outcome, expected) in cases {
            // When
            let miss = texture_of(outcome).err();

            // Then
            assert_eq!(miss, Some(expected));
        }
    }
}
