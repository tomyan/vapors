//! The two deadlines, proven with short ones: an app side that takes
//! longer than `start`'s, and a handler that blocks past `stop`'s. The
//! blocked threads end on their own when their sleep does.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{AppThread, AppThreadError, Deadlines};

const SHORT: Duration = Duration::from_millis(20);
/// Longer than `SHORT` by enough that a loaded machine still misses it.
const BLOCK: Duration = Duration::from_millis(300);

fn ignore(_: &str) {}

#[test]
fn an_app_side_slower_than_the_start_deadline_is_refused_naming_it() {
    // Given
    let deadlines = Deadlines {
        start: SHORT,
        stop: SHORT,
    };

    // When
    let started = AppThread::<()>::start_within(
        || {
            std::thread::sleep(BLOCK);
            |(): ()| {}
        },
        ignore,
        deadlines,
    );

    // Then
    let refused = started.err().expect("start refuses");
    assert_eq!(
        refused,
        AppThreadError::Deadline {
            what: "build the app side",
            after: SHORT
        }
    );
}

#[test]
fn a_handler_blocking_past_the_stop_deadline_is_refused_naming_it() {
    // Given: a handler that is blocking when stop is called
    let deadlines = Deadlines {
        start: Duration::from_secs(5),
        stop: SHORT,
    };
    let app = AppThread::start_within(|| |(): ()| std::thread::sleep(BLOCK), ignore, deadlines)
        .expect("the thread starts");
    app.send(()).expect("the thread runs");
    let started = Instant::now();
    while app.turns() == 0 {
        assert!(
            started.elapsed() < BLOCK,
            "the message's turn did not begin"
        );
        std::thread::sleep(Duration::from_millis(1));
    }

    // When
    let stopped = app.stop();

    // Then
    assert_eq!(
        stopped,
        Err(AppThreadError::Deadline {
            what: "stop",
            after: SHORT
        })
    );
}

/// Blocks for `BLOCK` when dropped, as app-authored `Drop` code may.
struct SlowDrop;

impl Drop for SlowDrop {
    fn drop(&mut self) {
        std::thread::sleep(BLOCK);
    }
}

#[test]
fn a_pending_task_whose_drop_blocks_past_the_stop_deadline_is_refused_naming_it() {
    // Given: a task, pending forever, holding a value whose drop blocks;
    // the thread drops it on its way out, after its last turn
    let deadlines = Deadlines {
        start: Duration::from_secs(5),
        stop: SHORT,
    };
    let make = || {
        |(): ()| {
            let held = SlowDrop;
            // Deadline::User: a test task that never ends; the thread drops it
            vapors_signals::spawn(vapors_signals::Deadline::User, async move {
                let _held = held;
                std::future::pending::<Result<(), String>>().await
            });
        }
    };
    let app = AppThread::start_within(make, ignore, deadlines).expect("the thread starts");
    app.send(()).expect("the thread runs");
    let started = Instant::now();
    while app.turns() < 2 {
        assert!(started.elapsed() < BLOCK, "the task's turn did not run");
        std::thread::sleep(Duration::from_millis(1));
    }

    // When
    let stopped = app.stop();

    // Then
    assert_eq!(
        stopped,
        Err(AppThreadError::Deadline {
            what: "stop",
            after: SHORT
        })
    );
}

/// A `SlowDrop` that records when its drop began.
struct TimedSlowDrop(Arc<Mutex<Option<Instant>>>);

impl Drop for TimedSlowDrop {
    fn drop(&mut self) {
        *self.0.lock().expect("no test thread panics holding it") = Some(Instant::now());
        std::thread::sleep(BLOCK);
    }
}

#[test]
fn a_start_whose_thread_ends_slowly_is_refused_naming_the_whole_deadline() {
    // Given: an app side that leaves a task holding a slow drop and
    // panics at once. The thread drops the task only after `ready` is
    // gone, which is what sends start down this path; its drop then
    // outlasts what is left of the deadline
    let deadlines = Deadlines {
        start: SHORT,
        stop: SHORT,
    };
    let dropping = Arc::new(Mutex::new(None));
    let held = TimedSlowDrop(Arc::clone(&dropping));
    let make = move || -> fn(()) {
        // Deadline::User: a test task that never ends; the thread drops it
        vapors_signals::spawn(vapors_signals::Deadline::User, async move {
            let _held = held;
            std::future::pending::<Result<(), String>>().await
        });
        panic!("the app side fails after spawning");
    };

    // When
    let started = AppThread::<()>::start_within(make, ignore, deadlines);
    let returned = Instant::now();

    // Then: the whole deadline, reached by waiting for the thread's end.
    // Start's Timeout arm names the same deadline; it is taken while the
    // thread still holds `ready`, so before any drop began
    assert_eq!(
        started.err(),
        Some(AppThreadError::Deadline {
            what: "build the app side",
            after: SHORT
        })
    );
    let began = *dropping.lock().expect("the drop does not panic");
    assert!(
        began.is_some_and(|began| began < returned),
        "start returned before the thread began dropping its tasks, so by its Timeout arm, \
         not by waiting for the thread's end; fix: what delayed the thread past {SHORT:?}; \
         reproduce: cargo nextest run -p vapors-app --lib \
         a_start_whose_thread_ends_slowly_is_refused_naming_the_whole_deadline"
    );
}
