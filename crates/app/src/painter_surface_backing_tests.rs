use vapors_gpu_harness::gpu;

use std::time::Duration;

use vapors_platform::Instant;

use super::super::scope::{DEADLINE, LoopWaker};
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
    let waker = LoopWaker::default();
    mounted.scopes.wait_for(
        Box::pin(std::future::pending()),
        Turn { waker: &waker, now },
    );
    [before, (mounted.wants_frame(), mounted.deadline())]
}

/// A mounted surface, needing no GPU, with a scope that never resolves
/// begun at `start`.
fn unseen_waiting(start: Instant) -> Mounted {
    let mut mounted = Mounted::new(Box::new(Idle));
    let waker = LoopWaker::default();
    let turn = Turn {
        waker: &waker,
        now: start,
    };
    mounted
        .scopes
        .wait_for(Box::pin(std::future::pending()), turn);
    mounted
}

#[test]
fn a_surface_no_frame_names_keeps_its_scope_until_its_deadline() {
    // Given
    let start = Instant::now();
    let mut mounted = unseen_waiting(start);

    // When
    mounted.begin_frame();
    mounted.end_frame(start + DEADLINE.saturating_sub(Duration::from_millis(1)));

    // Then
    assert_eq!(mounted.deadline(), Some(start + DEADLINE));
}

#[test]
fn a_surface_no_frame_names_gives_up_its_scope_at_the_deadline_and_is_refused_later() {
    // Given
    let start = Instant::now();
    let mut mounted = unseen_waiting(start);

    // When
    mounted.begin_frame();
    mounted.end_frame(start + DEADLINE);

    // Then
    assert_eq!(mounted.deadline(), None);
    let refusal = mounted.scopes.check(start + DEADLINE);
    let refusal = format!("{:?}", refusal.expect_err("refused when painted again"));
    assert!(refusal.contains("within 1000 ms"), "{refusal}");
}
