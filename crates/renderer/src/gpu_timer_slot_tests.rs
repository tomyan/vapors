use std::sync::mpsc;
use std::time::Duration;

use super::{Finished, MapOutcome, SlotState, missing};
use crate::gpu_time::{GpuMissing, GpuReading};

fn mapping(frame: u64) -> (SlotState, mpsc::Sender<MapOutcome>) {
    let (sender, outcome) = mpsc::channel();
    let state = SlotState::Mapping {
        frame,
        pairs: 2,
        outcome,
    };
    (state, sender)
}

#[test]
fn a_mapping_not_yet_done_is_not_finished_and_stays() {
    // Given
    let (mut slot, _sender) = mapping(7);

    // When
    let finished = slot.take_finished();

    // Then
    assert_eq!(finished, Finished::Nothing);
    assert!(matches!(slot, SlotState::Mapping { frame: 7, .. }));
}

#[test]
fn a_done_mapping_is_handed_over_for_reading_and_the_slot_freed() {
    // Given
    let (mut slot, sender) = mapping(7);
    sender.send(Ok(())).expect("the slot holds the receiver");

    // When
    let finished = slot.take_finished();

    // Then
    assert_eq!(finished, Finished::Mapped { frame: 7, pairs: 2 });
    assert!(matches!(slot, SlotState::Free));
}

#[test]
fn a_refused_map_is_a_missing_reading_naming_the_refusal() {
    // Given
    let (mut slot, sender) = mapping(7);
    sender
        .send(Err(wgpu::BufferAsyncError))
        .expect("the slot holds the receiver");

    // When
    let finished = slot.take_finished();

    // Then
    let Finished::Reading(GpuReading {
        frame: 7,
        time: Err(GpuMissing::MapFailed(_)),
        ..
    }) = finished
    else {
        panic!("{finished:?}");
    };
}

#[test]
fn a_mapping_still_pending_when_its_slot_is_needed_is_missing_past_its_deadline() {
    // Given
    let (mut slot, _sender) = mapping(7);

    // When
    let reclaimed = slot.reclaim(4);

    // Then
    assert_eq!(
        reclaimed,
        Some(missing(7, 2, GpuMissing::Deadline { frames: 4 }))
    );
    assert!(matches!(slot, SlotState::Free));
}

#[test]
fn a_reading_known_without_mapping_is_handed_over_once() {
    // Given
    let reading = GpuReading {
        frame: 3,
        pairs: 0,
        time: Ok(Duration::ZERO),
    };
    let mut slot = SlotState::Done(reading.clone());

    // When
    let first = slot.take_finished();
    let second = slot.take_finished();

    // Then
    assert_eq!(
        (first, second),
        (Finished::Reading(reading), Finished::Nothing)
    );
}

#[test]
#[should_panic(expected = "still open 4 frames later")]
fn reclaiming_a_slot_still_recorded_into_panics() {
    SlotState::Recording { frame: 1 }.reclaim(4);
}
