//! The frame clock a script reads: `frame_time()` and `frame_delta()`,
//! two app-wide signals the app advances once at the start of each
//! frame (`advance_frame_clock`). A script that reads either keeps the
//! loop drawing while a reader is live (`frame_clock_wanted`); a page
//! that never calls them has no clock, and the app draws as before.

use std::cell::RefCell;
use std::time::Duration;

use vapors_signals::{ReadSignal, Signal, unowned};

/// The clock's signals and the frame times that make them.
#[derive(Clone, Copy)]
struct Clock {
    time: Signal<Duration>,
    delta: Signal<Duration>,
    /// The `at` of the first frame after the clock was made.
    first: Option<Duration>,
    last: Option<Duration>,
}

thread_local! {
    static CLOCK: RefCell<Option<Clock>> = const { RefCell::new(None) };
}

/// The clock, made on the first call. Its signals belong to no owner:
/// the component that happens to read first must not take it with it.
fn clock() -> Clock {
    CLOCK.with(|slot| {
        *slot.borrow_mut().get_or_insert_with(|| {
            unowned(|| Clock {
                time: Signal::named("frame_time", Duration::ZERO),
                delta: Signal::named("frame_delta", Duration::ZERO),
                first: None,
                last: None,
            })
        })
    })
}

/// The time since the first frame that followed the first call, set
/// once at the start of each frame. Every call returns a read-only view of the same signal.
#[must_use]
pub fn frame_time() -> ReadSignal<Duration> {
    clock().time.read_only()
}

/// The time since the previous frame: zero on the first. After a wait
/// (`Next::Wait`) it includes the wait. Every call returns a read-only view of the same signal.
#[must_use]
pub fn frame_delta() -> ReadSignal<Duration> {
    clock().delta.read_only()
}

/// Sets the clock for the frame that starts `at` (any steady epoch, the
/// same each frame). Does nothing until a script has called
/// `frame_time` or `frame_delta`.
pub fn advance_frame_clock(at: Duration) {
    let Some(mut now) = CLOCK.with(|slot| *slot.borrow()) else {
        return;
    };
    let first = *now.first.get_or_insert(at);
    let delta = now
        .last
        .map_or(Duration::ZERO, |last| at.saturating_sub(last));
    now.last = Some(at);
    CLOCK.with(|slot| *slot.borrow_mut() = Some(now));
    now.delta.set(delta);
    now.time.set(at.saturating_sub(first));
}

/// Whether a live reader depends on either signal: the app draws every
/// frame while one does.
#[must_use]
pub fn frame_clock_wanted() -> bool {
    CLOCK.with(|slot| {
        slot.borrow()
            .is_some_and(|c| c.time.has_subscribers() || c.delta.has_subscribers())
    })
}
