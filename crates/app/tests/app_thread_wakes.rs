//! Wakes on the application thread (`docs/plans/desktop-app-thread.md`,
//! L1a): wakes before a turn cost one turn, a wake from another thread
//! reaches an idle thread, and wakes from another thread while the thread
//! polls are never lost.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use vapors_app::AppThread;
use vapors_signals::{Deadline, spawn};

use super::app_thread::{ignore, wait_for};

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

#[test]
fn wakes_from_another_thread_while_the_thread_polls_are_none_of_them_lost() {
    // Given: a task fed by another thread, and a busy task that keeps each
    // round of polls long, so sends land while the thread is polling
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<u32>();
    let arrived = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&arrived);
    let mut receiver = Some(receiver);
    let app = AppThread::start(
        move || {
            move |(): ()| {
                let receiver = receiver.take().expect("one message");
                spawn_receiver(receiver, Arc::clone(&counted));
                // Deadline::User: test tasks that end with the thread
                spawn(Deadline::User, Busy);
            }
        },
        ignore,
    )
    .expect("the thread starts");
    app.send(()).expect("the thread runs");

    // When: a thousand sends from off the thread, each a wake
    std::thread::spawn(move || {
        for n in 0..1000 {
            sender.send(n).unwrap();
            std::thread::sleep(Duration::from_micros(10));
        }
    });

    // Then
    wait_for("all thousand", || arrived.load(Ordering::SeqCst) == 1000);
    app.stop().expect("the thread stops");
}

#[test]
fn a_wake_from_another_thread_reaches_an_idle_thread() {
    // Given: an idle thread whose only task waits on a channel
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<u32>();
    let arrived = Arc::new(AtomicU64::new(0));
    let counted = Arc::clone(&arrived);
    let waiting = Arc::new(AtomicBool::new(false));
    let parked = Arc::clone(&waiting);
    let mut receiver = Some(receiver);
    let app = AppThread::start(
        move || {
            move |(): ()| {
                let receiver = receiver.take().expect("one message");
                spawn_waiting(receiver, Arc::clone(&counted), Arc::clone(&parked));
            }
        },
        ignore,
    )
    .expect("the thread starts");
    app.send(()).expect("the thread runs");
    // Waiting, not the turn count: the count rises as a turn begins, and
    // a send landing during the receiver's first poll is taken unwoken
    wait_for("the receiver to wait", || waiting.load(Ordering::SeqCst));
    assert_eq!(app.turns(), 2, "the message and the spawn's wake");

    // When: one send from off the thread
    std::thread::spawn(move || sender.send(1).unwrap())
        .join()
        .unwrap();

    // Then: the receiver continues, in one more turn
    wait_for("the value", || arrived.load(Ordering::SeqCst) == 1);
    assert_eq!(app.turns(), 3, "one wake cost other than one turn");
    app.stop().expect("the thread stops");
}

/// A task that takes one value, setting `waiting` once its receiver's
/// waker is registered and it returns `Pending`.
fn spawn_waiting(
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<u32>,
    arrived: Arc<AtomicU64>,
    waiting: Arc<AtomicBool>,
) {
    // Deadline::User: a test task that ends on its value
    spawn(Deadline::User, async move {
        let value = std::future::poll_fn(|cx| {
            let polled = receiver.poll_recv(cx);
            waiting.store(polled.is_pending(), Ordering::SeqCst);
            polled
        })
        .await;
        if value.is_some() {
            arrived.fetch_add(1, Ordering::SeqCst);
        }
        Ok::<(), String>(())
    });
}

fn spawn_receiver(
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<u32>,
    arrived: Arc<AtomicU64>,
) {
    spawn(Deadline::User, async move {
        while receiver.recv().await.is_some() {
            arrived.fetch_add(1, Ordering::SeqCst);
        }
        Ok::<(), String>(())
    });
}

/// Wakes itself and blocks 100 µs on every poll.
struct Busy;

impl std::future::Future for Busy {
    type Output = Result<(), String>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::thread::sleep(Duration::from_micros(100));
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    }
}
