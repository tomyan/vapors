use vapors_gpu_harness::gpu;

use super::{Backing, Mounted};
use crate::surface::{SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};

struct Idle;

impl SurfaceRenderer for Idle {
    fn render(&mut self, _frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        Ok(())
    }
}

fn idle_mounted_with_backing() -> Mounted {
    let gpu = gpu();
    let size = SurfaceSize {
        width: 4,
        height: 4,
        scale: 1.0,
    };
    let mut mounted = Mounted::new(Box::new(Idle));
    mounted.fresh = false;
    mounted.backing = Some(Backing::new(&gpu.device, size));
    mounted
}

/// `wants_frame` of an idle, backed surface before and after a scope
/// that never resolves is added. Called from `gpu_surface_backing_tests`.
pub(crate) fn wants_frame_around_a_pending_scope() -> (bool, bool) {
    let mut mounted = idle_mounted_with_backing();
    let before = mounted.wants_frame();
    mounted.scopes.wait_for(Box::pin(std::future::pending()));
    (before, mounted.wants_frame())
}
