use vapors_platform::Instant;

use super::super::scope::fakes::{Late, counting_waker, woken};
use super::{Mounted, Surfaces};
use crate::painter::surfaces::Turn;
use crate::surface::{SurfaceError, SurfaceFrame, SurfaceRenderer};

struct Idle;

impl SurfaceRenderer for Idle {
    fn render(&mut self, _frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        Ok(())
    }
}

/// Surfaces with one surface waiting on `late`, begun with no waker.
fn waiting_on(late: &Late) -> Surfaces {
    let mut surfaces = Surfaces::default();
    let mut mounted = Mounted::new(Box::new(Idle));
    let turn = Turn {
        waker: &surfaces.waker,
        now: Instant::now(),
    };
    mounted.scopes.wait_for(Box::pin(late.clone()), turn);
    surfaces.mounted.insert("track".to_owned(), mounted);
    surfaces
}

#[test]
fn an_error_that_arrived_before_the_waker_was_given_wakes_it_once() {
    // Given
    let late = Late::default();
    let mut surfaces = waiting_on(&late);
    let (count, waker) = counting_waker();

    // When
    late.resolve(Some("bad bind group"));
    surfaces.wake_with(waker);

    // Then
    assert_eq!(woken(&count), 1);
}

#[test]
fn a_scope_unresolved_or_clean_when_the_waker_is_given_wakes_nothing() {
    for resolved in [false, true] {
        // Given
        let late = Late::default();
        let mut surfaces = waiting_on(&late);
        let (count, waker) = counting_waker();

        // When
        if resolved {
            late.resolve(None);
        }
        surfaces.wake_with(waker);

        // Then
        assert_eq!(woken(&count), 0, "resolved: {resolved}");
    }
}
