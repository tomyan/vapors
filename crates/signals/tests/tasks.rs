//! The owner-scoped ready-task primitive (async-components.md T1): a
//! task is polled only when woken, and dies with its owner.

mod common;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::{fresh, gate, hook};
use vapors_signals::{Deadline, Owner, Signal, run_ready, spawn};

#[test]
fn a_spawn_alone_calls_the_hook() {
    // Given
    let wakes = hook();
    let (slot, done, polls) = fresh();
    spawn(Deadline::User, gate(&slot, &done, &polls));
    // Then: the loop was asked to run, and the task has not run itself
    assert_eq!((wakes.load(Ordering::SeqCst), polls.get()), (1, 0));
    assert_eq!(run_ready().polled, 1);
}

#[test]
fn no_poll_before_a_wake() {
    // Given
    let wakes = hook();
    let (slot, done, polls) = fresh();
    spawn(Deadline::User, gate(&slot, &done, &polls));
    assert_eq!(polls.get(), 0, "spawn must not poll");
    assert_eq!(run_ready().polled, 1, "the first poll is queued by spawn");
    // When: nothing wakes it
    let idle = run_ready();
    // Then
    assert_eq!((idle.polled, polls.get()), (0, 1));
    // When: it is woken
    done.set(true);
    slot.borrow_mut().take().unwrap().wake();
    let ran = run_ready();
    assert_eq!(
        (wakes.load(Ordering::SeqCst), ran.polled, ran.completed),
        (2, 1, 1)
    );
    assert_eq!(run_ready().polled, 0, "a completed task is gone");
}

#[test]
fn a_waker_woken_from_another_thread_runs_the_task() {
    // Given
    let wakes = hook();
    let (slot, done, polls) = fresh();
    spawn(Deadline::User, gate(&slot, &done, &polls));
    let _ = run_ready();
    done.set(true);
    let sender = slot.borrow_mut().take().unwrap();
    std::thread::spawn(move || sender.wake()).join().unwrap();
    // Then
    let ran = run_ready();
    assert_eq!(
        (wakes.load(Ordering::SeqCst), ran.polled, ran.completed),
        (2, 1, 1)
    );
}

#[test]
fn disposal_before_completion_drops_the_future_unpolled() {
    // Given
    hook();
    let (slot, done, polls) = fresh();
    let (dropped, shown) = (Rc::new(Cell::new(false)), Signal::new(0));
    let owner = Owner::new();
    owner.with(|| {
        let guard = common::Guard(Rc::clone(&dropped));
        let inner = gate(&slot, &done, &polls);
        spawn(Deadline::User, async move {
            let _guard = guard;
            inner.await?;
            shown.set(1);
            Ok::<(), String>(())
        });
    });
    let _ = run_ready();
    // When
    owner.dispose();
    done.set(true);
    slot.borrow_mut().take().unwrap().wake();
    // Then
    assert!(dropped.get(), "disposal drops the future");
    assert_eq!((run_ready().polled, polls.get(), shown.get()), (0, 1, 0));
}

#[test]
fn disposal_before_the_first_poll_never_polls() {
    // Given
    hook();
    let (slot, done, polls) = fresh();
    let owner = Owner::new();
    owner.with(|| spawn(Deadline::User, gate(&slot, &done, &polls)));
    // When
    owner.dispose();
    // Then
    assert_eq!((run_ready().polled, polls.get()), (0, 0));
}

#[test]
fn an_err_comes_back_naming_the_spawn_site() {
    // Given
    hook();
    spawn(Deadline::User, async { Err::<(), _>("disk full") });
    // When
    let ran = run_ready();
    // Then
    let [message] = &ran.failures[..] else {
        panic!("one failure expected: {ran:?}")
    };
    assert!(
        message.contains("tests/tasks.rs:") && message.contains("disk full"),
        "{message}"
    );
    assert_eq!(ran.completed, 1);
}

#[test]
fn the_deadline_is_carried_and_not_judged_yet() {
    hook();
    let want = Deadline::Measured(Duration::from_secs(1));
    let task = spawn(want, std::future::pending::<Result<(), String>>());
    // When
    let _ = (run_ready(), run_ready());
    assert_eq!(task.deadline(), Some(want));
}
