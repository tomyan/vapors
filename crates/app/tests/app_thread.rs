//! The desktop application thread (`docs/plans/desktop-app-thread.md`,
//! L1a), with no window: handlers and every continuation after an
//! `.await` run on it, a Tokio timer wakes it, and with nothing pending
//! it takes no turns. Timings use the real clock with wide bounds:
//! Tokio's paused clock advances by itself whenever the runtime is idle.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use vapors_app::AppThread;
use vapors_signals::{Deadline, spawn};

/// Every wait in these tests: the longest a passing wait takes is a
/// 10 ms sleep or a few ticks, so 5 s is reached only by a hang.
pub const WAIT: Duration = Duration::from_secs(5);

pub fn ignore(_: &str) {}

/// Waits until `done` holds, failing the test at `WAIT`.
pub fn wait_for(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < WAIT,
            "{what} did not happen within {WAIT:?}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn here() -> ThreadId {
    std::thread::current().id()
}

#[test]
fn a_handler_runs_on_the_application_thread() {
    // Given
    let (seen, ran_on) = mpsc::channel();
    let app = AppThread::start(move || move |(): ()| seen.send(here()).unwrap(), ignore)
        .expect("the thread starts");

    // When
    app.send(()).expect("the thread runs");

    // Then
    let id = ran_on.recv_timeout(WAIT).expect("the handler ran");
    assert_eq!(
        id,
        app.thread_id(),
        "the handler ran off the application thread"
    );
    assert_ne!(id, here());
    app.stop().expect("the thread stops");
}

#[test]
fn a_continuation_after_a_timer_runs_on_the_application_thread() {
    // Given: a handler that spawns a task awaiting Tokio's timer
    let (seen, ran_on) = mpsc::channel();
    let app = AppThread::start(
        move || {
            move |(): ()| {
                let seen = seen.clone();
                // Deadline::User: a test task that ends after its 10 ms sleep
                spawn(Deadline::User, async move {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    seen.send(here()).map_err(|e| e.to_string())
                });
            }
        },
        ignore,
    )
    .expect("the thread starts");

    // When
    app.send(()).expect("the thread runs");

    // Then
    let id = ran_on.recv_timeout(WAIT).expect("the timer woke the task");
    assert_eq!(
        id,
        app.thread_id(),
        "the continuation ran off the application thread"
    );
    app.stop().expect("the thread stops");
}

#[test]
fn with_nothing_pending_the_thread_takes_no_turns() {
    // Given: one message handled, nothing spawned
    let app = AppThread::start(|| |(): ()| {}, ignore).expect("the thread starts");
    app.send(()).expect("the thread runs");
    wait_for("the message's turn", || app.turns() == 1);

    // When
    std::thread::sleep(Duration::from_millis(100));

    // Then
    assert_eq!(app.turns(), 1, "an idle application thread woke");
    app.stop().expect("the thread stops");
}

#[test]
fn wakes_before_a_turn_cost_one_turn() {
    // Given: a handler that spawns ten tasks, each waking the thread
    let done = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&done);
    let app = AppThread::start(
        move || {
            move |(): ()| {
                for _ in 0..10 {
                    let done = Arc::clone(&counted);
                    // Deadline::User: a test task that ends on its first poll
                    spawn(Deadline::User, async move {
                        done.fetch_add(1, Ordering::SeqCst);
                        Ok::<(), String>(())
                    });
                }
            }
        },
        ignore,
    )
    .expect("the thread starts");

    // When
    app.send(()).expect("the thread runs");
    wait_for("the ten tasks", || done.load(Ordering::SeqCst) == 10);

    // Then: the message's turn and one turn for the ten wakes
    assert_eq!(
        app.turns(),
        2,
        "ten wakes before a turn cost more than one turn"
    );
    app.stop().expect("the thread stops");
}
