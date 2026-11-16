//! GPU tier: uniform slots (`uniform_slots.rs`) found by hash; the map
//! holds a key for each slot at most, and an empty table under a head is
//! its own value.

use vapors_gpu_harness::gpu;

use crate::{FrameEncoder, RowOffsets, UniformSlots, WorkCounter};

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

#[test]
fn gpu_an_empty_table_after_a_full_one_under_one_head_is_a_slot_of_its_own() {
    // Given
    let gpu = gpu();
    let mut slots: UniformSlots<{ 16 + RowOffsets::CAPACITY * 8 }> =
        UniformSlots::new(&gpu.device, "test");
    let work = WorkCounter::default();
    let encoder = FrameEncoder::new(&gpu.device);
    let rowed = RowOffsets::whole(&[[0.0, 4.0]]);

    // When: one head with a table, then with the empty one, then again
    let mut at = |rows: &RowOffsets| {
        let found = slots.slot_with_rows(&encoder, (&gpu.queue, &work), ([1; 16], rows));
        found.expect("room")
    };
    let first = at(&rowed);
    let empty = at(&RowOffsets::NONE);
    let again = at(&RowOffsets::NONE);

    // Then
    assert_ne!(first, empty);
    assert_eq!(empty, again);
    encoder.submit(&gpu.queue).expect("no refusal");
}
