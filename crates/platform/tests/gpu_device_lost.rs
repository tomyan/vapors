//! GPU tier: wgpu's device-lost callback reaches vapors' lost state.
//! `Device::destroy` loses the device on purpose, as a backgrounded tab
//! or a GPU reset does by accident.

use vapors_gpu_harness::gpu;
use vapors_platform::DeviceLost;

#[test]
fn gpu_destroying_the_device_records_the_loss_with_wgpus_reason() {
    // Given: a device of its own, watched
    let gpu = gpu();
    let state = DeviceLost::default();
    state.watch(&gpu.device, || {});
    assert_eq!(state.get(), None, "not lost before destroy");

    // When
    gpu.device.destroy();
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());

    // Then
    let loss = state.get().expect("the callback recorded the loss");
    assert_eq!(loss.reason, "Destroyed", "{loss:?}");
}
