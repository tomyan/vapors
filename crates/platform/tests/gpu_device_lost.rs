//! GPU tier: wgpu's device-lost callback reaches vapors' lost state and
//! wakes the loop. `Device::destroy` loses the device on purpose, as a
//! backgrounded tab or a GPU reset does by accident; dropping a device
//! is not a loss.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vapors_gpu_harness::gpu;
use vapors_platform::DeviceLost;

/// A waker that records that it was called.
fn waker() -> (Arc<AtomicBool>, impl Fn() + Send + 'static) {
    let woken = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&woken);
    (woken, move || flag.store(true, Ordering::SeqCst))
}

#[test]
fn gpu_destroying_the_device_records_the_loss_and_wakes_the_loop() {
    // Given: a device of its own, watched
    let gpu = gpu();
    let state = DeviceLost::default();
    let (woken, wake) = waker();
    state.watch(&gpu.device, wake);
    assert_eq!(state.get(), None, "not lost before destroy");

    // When
    gpu.device.destroy();
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());

    // Then
    let loss = state.get().expect("the callback recorded the loss");
    assert_eq!(loss.reason, "Destroyed", "{loss:?}");
    assert!(woken.load(Ordering::SeqCst), "the waker was called");
}

#[test]
fn gpu_dropping_the_device_is_never_reported_as_a_loss() {
    // Given
    let gpu = gpu();
    let state = DeviceLost::default();
    let (woken, wake) = waker();
    state.watch(&gpu.device, wake);

    // When: the device and its queue go away without destroy
    drop(gpu);

    // Then
    assert_eq!(state.get(), None);
    assert!(!woken.load(Ordering::SeqCst), "no wake for a drop");
}
