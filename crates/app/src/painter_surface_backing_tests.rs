use vapors_gpu_harness::gpu;

use vapors_platform::Instant;

use super::{Backing, Mounted, Turn};
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

/// `wants_frame` and `deadline` of an idle, backed surface before and
/// after a scope that never resolves is added at `now`. Called from
/// `gpu_surface_backing_tests`.
pub(crate) fn wants_frame_around_a_pending_scope(now: Instant) -> [(bool, Option<Instant>); 2] {
    let mut mounted = idle_mounted_with_backing();
    let before = (mounted.wants_frame(), mounted.deadline());
    let turn = Turn {
        waker: std::task::Waker::noop(),
        now,
    };
    mounted
        .scopes
        .wait_for(Box::pin(std::future::pending()), turn);
    [before, (mounted.wants_frame(), mounted.deadline())]
}
