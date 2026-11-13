//! Instruction samples keep each phase's counts aligned with the frames.

use std::collections::BTreeMap;

use super::InstructionSamples;

fn phases(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|&(name, n)| (name.to_owned(), n))
        .collect()
}

#[test]
fn a_phase_missing_from_a_frame_counts_zero_for_it() {
    // Given
    let mut samples = InstructionSamples::default();

    // When
    samples.push(10, phases(&[("draw", 4)]));
    samples.push(12, phases(&[("update", 5)]));

    // Then
    assert_eq!(samples.frame, [10, 12]);
    assert_eq!(samples.phases["draw"], [4, 0]);
    assert_eq!(samples.phases["update"], [0, 5]);
}

#[test]
fn every_nth_frame_keeps_those_frames_and_their_phases() {
    // Given: frames costing 10 to 15, an update in each
    let mut samples = InstructionSamples::default();
    for count in 10..16 {
        samples.push(count, phases(&[("update", count + 100)]));
    }

    // When
    let odd = samples.every_nth(1, 2);

    // Then
    assert_eq!(odd.frame, [11, 13, 15]);
    assert_eq!(odd.phases["update"], [111, 113, 115]);
}
