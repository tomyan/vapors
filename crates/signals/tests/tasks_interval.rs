//! The Live interval as an ordinary `.await` loop in an owner-scoped
//! task over a controlled clock (async-components.md T1, the Timer
//! row): 100 ms cadence, one catch-up after a stall, nothing after
//! cancel or disposal, one interval per enable.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use common::{Clock, hook};
use vapors_signals::{Deadline, Owner, Signal, run_ready, spawn};

const PERIOD: Duration = Duration::from_millis(100);

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// The dashboard's loop: show `source()` once per period; after a
/// stall the next deadline is a full period from now (no burst).
fn live(clock: &Clock, shown: Signal<u64>, source: Clock) {
    let clock = clock.clone();
    spawn(Deadline::User, async move {
        let mut at = clock.now() + PERIOD;
        loop {
            clock.sleep_until(at).await;
            shown.set(u64::try_from(source.now().as_millis()).unwrap());
            at = clock.now() + PERIOD;
        }
        #[allow(unreachable_code)]
        Ok::<(), String>(())
    });
}

fn enable(clock: &Clock, shown: Signal<u64>) -> Owner {
    HOOK.with(|_| ());
    let owner = Owner::new();
    owner.with(|| live(clock, shown, clock.clone()));
    let _ = run_ready();
    owner
}

thread_local! {
    static HOOK: Arc<AtomicUsize> = hook();
}

fn wakes() -> Arc<AtomicUsize> {
    HOOK.with(Arc::clone)
}

#[test]
fn a_100ms_tick_shows_the_current_value() {
    let (clock, shown) = (Clock::default(), Signal::new(0));
    let _owner = enable(&clock, shown);
    assert_eq!(clock.earliest(), Some(PERIOD));
    clock.advance_to(ms(99));
    assert_eq!((run_ready().polled, shown.get()), (0, 0));
    clock.advance_to(ms(100));
    assert_eq!((run_ready().polled, shown.get()), (1, 100));
    assert_eq!(clock.earliest(), Some(ms(200)), "the next tick is armed");
}

#[test]
fn a_late_tick_shows_once_and_resumes_the_cadence() {
    let (clock, shown) = (Clock::default(), Signal::new(0));
    let _owner = enable(&clock, shown);
    clock.advance_to(ms(750));
    assert_eq!(run_ready().polled, 1);
    assert_eq!(shown.get(), 750, "current value, one update");
    assert_eq!(clock.scheduled(), 1);
    assert_eq!(clock.earliest(), Some(ms(850)));
    clock.advance_to(ms(850));
    let _ = run_ready();
    assert_eq!(shown.get(), 850);
}

#[test]
fn disposal_cancels_even_with_a_wake_already_queued() {
    let (clock, shown) = (Clock::default(), Signal::new(0));
    let wakes = wakes();
    let owner = enable(&clock, shown);
    clock.advance_to(ms(100));
    let queued = wakes.load(Ordering::SeqCst);
    assert!(queued >= 2, "the wake is queued before disposal");
    owner.dispose();
    assert_eq!((run_ready().polled, shown.get()), (0, 0));
    assert_eq!(clock.scheduled(), 0, "no timer left armed");
    clock.advance_to(ms(1000));
    assert_eq!(wakes.load(Ordering::SeqCst), queued, "no wake after");
}

#[test]
fn disposal_while_waiting_leaves_no_timer_and_no_update() {
    let (clock, shown) = (Clock::default(), Signal::new(0));
    let owner = enable(&clock, shown);
    owner.dispose();
    assert_eq!(clock.scheduled(), 0);
    clock.advance_to(ms(500));
    assert_eq!((run_ready().polled, shown.get()), (0, 0));
}

#[test]
fn enabling_again_arms_exactly_one_fresh_interval() {
    let (clock, shown) = (Clock::default(), Signal::new(0));
    let first = enable(&clock, shown);
    clock.advance_to(ms(100));
    let _ = run_ready();
    first.dispose();
    clock.advance_to(ms(130));
    let _second = enable(&clock, shown);
    assert_eq!(clock.scheduled(), 1, "one timer, not two");
    assert_eq!(clock.earliest(), Some(ms(230)), "fresh, from re-enable");
    clock.advance_to(ms(230));
    assert_eq!(run_ready().polled, 1, "one callback per tick");
    assert_eq!(shown.get(), 230);
}
