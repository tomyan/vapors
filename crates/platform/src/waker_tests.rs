//! A detached platform's waker raises the same request the loop takes;
//! the window's waker is checked by the `wake_window` example and the
//! page's `scripts/web-wake-frames.sh`.

use crate::Platform;

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
