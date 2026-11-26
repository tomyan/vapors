//! The two deadlines, proven with short ones: an app side that takes
//! longer than `start`'s, and a handler that blocks past `stop`'s. The
//! blocked threads end on their own when their sleep does.

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
