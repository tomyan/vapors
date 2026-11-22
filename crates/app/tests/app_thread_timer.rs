//! A Tokio interval on the application thread
//! (`docs/plans/desktop-app-thread.md`, L1a): it wakes the thread about
//! once per tick, and disposing its task's owner stops the ticks and the
//! turns. Real clock, wide bounds (see `app_thread.rs`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use vapors_app::AppThread;
use vapors_signals::{Deadline, Owner, spawn};

use super::app_thread::{ignore, wait_for};

/// The interval these tests run.
const TICK: Duration = Duration::from_millis(20);

enum Live {
    On,
    Off,
}

/// An app side whose `On` runs an interval counting into `ticks`, under
/// an owner that `Off` disposes.
fn live(ticks: Arc<AtomicU64>) -> impl FnMut(Live) {
    let mut running: Option<Owner> = None;
    move |message| match message {
        Live::On => {
            let owner = Owner::new();
            let ticks = Arc::clone(&ticks);
            // Deadline::User: runs until Off disposes its owner
            owner.with(|| spawn(Deadline::User, interval(ticks)));
            running = Some(owner);
        }
        Live::Off => running.take().expect("Off follows On").dispose(),
    }
}

async fn interval(ticks: Arc<AtomicU64>) -> Result<(), String> {
    let mut interval = tokio::time::interval(TICK);
    loop {
        interval.tick().await;
        ticks.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn a_running_interval_wakes_the_thread_about_once_per_tick() {
    // Given
    let ticks = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&ticks);
    let app = AppThread::start(move || live(counted), ignore).expect("the thread starts");

    // When
    app.send(Live::On).expect("the thread runs");
    std::thread::sleep(Duration::from_millis(200));

    // Then: the message's turn, the spawn's wake, then one per tick
    let (ticks, turns) = (ticks.load(Ordering::SeqCst), app.turns());
    assert!(ticks >= 5, "{ticks} ticks of {TICK:?} in 200 ms");
    assert!(turns <= ticks + 3, "{turns} turns for {ticks} ticks");
    app.stop().expect("the thread stops");
}

#[test]
fn disposing_the_intervals_owner_stops_its_ticks_and_the_turns() {
    // Given: a running interval, then Off
    let ticks = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&ticks);
    let app = AppThread::start(move || live(counted), ignore).expect("the thread starts");
    app.send(Live::On).expect("the thread runs");
    wait_for("two ticks", || ticks.load(Ordering::SeqCst) >= 2);
    app.send(Live::Off).expect("the thread runs");
    std::thread::sleep(TICK * 2);
    let (ticks_then, turns_then) = (ticks.load(Ordering::SeqCst), app.turns());

    // When
    std::thread::sleep(Duration::from_millis(100));

    // Then
    assert_eq!(
        ticks.load(Ordering::SeqCst),
        ticks_then,
        "a disposed interval ticked"
    );
    assert_eq!(
        app.turns(),
        turns_then,
        "the thread woke after the interval was disposed"
    );
    app.stop().expect("the thread stops");
}

#[test]
fn the_app_side_may_make_a_tokio_timer_while_it_is_built() {
    // When: the app side makes its interval before any message
    let started = AppThread::start(
        || {
            let mut interval = tokio::time::interval(TICK);
            move |(): ()| {
                let _ = &mut interval;
            }
        },
        ignore,
    );

    // Then: built inside the runtime's context, so no panic
    started
        .expect("the app side was built")
        .stop()
        .expect("the thread stops");
}
