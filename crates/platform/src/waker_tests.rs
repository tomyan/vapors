//! A detached platform's waker raises the same request the loop takes;
//! the window's waker is checked by the `wake_window` example and the
//! page's `scripts/web-wake-frames.sh`. The loop's answer to each kind of
//! wake is `deliver`'s, called by the handler with the posted kind.

use std::cell::Cell;

use super::{Wake, deliver};
use crate::{App, Event, Frame, Next, Platform};

#[test]
fn a_wake_asks_for_one_frame() {
    // Given
    let platform = Platform::detached();
    let waker = platform.waker();

    // When
    waker.wake().expect("detached");
    waker.wake().expect("detached");

    // Then
    assert!(platform.take_redraw_request(), "a wake asks for a frame");
    assert!(
        !platform.take_redraw_request(),
        "two wakes before it draw one"
    );
}

#[test]
fn a_clone_wakes_the_same_platform() {
    // Given
    let platform = Platform::detached();
    let clone = platform.waker().clone();

    // When
    clone.wake().expect("detached");

    // Then
    assert!(platform.take_redraw_request());
}

#[test]
fn no_wake_asks_for_nothing() {
    // Given
    let platform = Platform::detached();
    let _waker = platform.waker();

    // Then
    assert!(!platform.take_redraw_request());
}

#[test]
fn natively_a_waker_can_go_to_another_thread() {
    // Given
    let platform = Platform::detached();
    let waker = platform.waker();

    // When
    std::thread::spawn(move || waker.wake())
        .join()
        .expect("the thread ran")
        .expect("detached");

    // Then
    assert!(platform.take_redraw_request());
}

#[test]
fn a_task_waker_raises_a_task_wake_and_no_frame() {
    // Given
    let platform = Platform::detached();

    // When
    platform.task_waker().wake().expect("detached");

    // Then
    assert!(platform.take_task_wake(), "the task waker woke nothing");
    assert!(
        !platform.take_redraw_request(),
        "a task wake asked for a frame"
    );
}

#[test]
fn a_frame_waker_raises_no_task_wake() {
    // Given
    let platform = Platform::detached();

    // When
    platform.waker().wake().expect("detached");

    // Then
    assert!(!platform.take_task_wake());
}

/// Counts `tasks_woken` calls, and asks for no frame from them.
#[derive(Default)]
struct TaskCounting {
    tasks_woken: Cell<u32>,
}

impl App for TaskCounting {
    fn event(&mut self, _: Event, _: &Platform) {}

    fn frame(&mut self, _: &Frame<'_>) -> Next {
        unreachable!("no frame is drawn without a window")
    }

    fn tasks_woken(&mut self, _: &Platform) {
        self.tasks_woken.set(self.tasks_woken.get() + 1);
    }
}

#[test]
fn a_frame_wake_asks_for_a_frame_without_reaching_the_app() {
    // Given: surfaces, pastes and GPU scopes wake for a frame
    let (mut app, platform) = (TaskCounting::default(), Platform::detached());

    // When
    deliver(Wake::Frame, &mut app, &platform);

    // Then
    assert!(platform.take_redraw_request(), "a frame wake drew no frame");
    assert_eq!(app.tasks_woken.get(), 0, "a frame wake polled the tasks");
}

#[test]
fn a_task_wake_reaches_the_app_and_asks_for_no_frame_itself() {
    // Given
    let (mut app, platform) = (TaskCounting::default(), Platform::detached());

    // When
    deliver(Wake::Tasks, &mut app, &platform);

    // Then: the app decides whether a frame is owed
    assert_eq!(app.tasks_woken.get(), 1);
    assert!(!platform.take_redraw_request());
}
