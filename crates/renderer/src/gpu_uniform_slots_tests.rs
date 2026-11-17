//! GPU tier: uniform slots (`uniform_slots.rs`) found by hash; the map
//! holds a key for each slot at most.

use vapors_gpu_harness::gpu;

use crate::{FrameEncoder, UniformSlots, WorkCounter};

#[test]
fn gpu_a_value_written_into_a_reused_slot_leaves_no_key_behind() {
    // Given: slots for 16-byte values
    let gpu = gpu();
    let mut slots: UniformSlots<16> = UniformSlots::new(&gpu.device, "test");
    let work = WorkCounter::default();

    // When: 200 frames, each with a value never seen before, as a smooth
    // scroll writes a new shift every frame
    for frame in 0..200u32 {
        let encoder = FrameEncoder::new(&gpu.device);
        let mut value = [0; 16];
        value[..4].copy_from_slice(&frame.to_le_bytes());
        slots
            .slot(&encoder, &gpu.queue, &work, value)
            .expect("room");
        encoder.submit(&gpu.queue).expect("no refusal");
    }

    // Then: a key for each slot at most
    let (keys, held) = slots.keys_and_slots();
    assert!(keys <= held, "{keys} keys for {held} slots");
}
